// Linux X11 backend for GlobalHotkeyManager.
//
// Uses the X RECORD extension to observe key presses globally without
// XGrabKey'ing every key (XRecord is non-intrusive: other clients still
// receive their own input). The recorder runs on a worker QThread; key
// events are forwarded to the main thread via a queued signal.
//
// For double-tap sequences (e.g. "Ctrl+C+C"), we additionally XGrabKey the
// second-key chord for ~500 ms after the first chord matches, so the second
// keypress is delivered to us alone (the foreground app's own Ctrl+C is the
// first press, which we don't grab so the user's copy still works).

#ifdef WITH_X11

#include "global_hotkey_manager.h"

#include <QGuiApplication>
#include <QTimer>

#include <X11/Xlib.h>
#include <X11/Xlibint.h>
#include <X11/keysym.h>
#include <X11/extensions/record.h>

// X11 leaks `Bool`, `Status`, `min`, `max` as macros. Undef them after the
// X11 headers so the rest of this TU (and any later Qt headers) is clean.
#ifdef Bool
  #undef Bool
#endif
#ifdef Status
  #undef Status
#endif
#ifdef min
  #undef min
#endif
#ifdef max
  #undef max
#endif

namespace {
inline ::Display* asDisplay(void* p)         { return reinterpret_cast<::Display*>(p); }
inline XRecordRange* asRange(void* p)         { return reinterpret_cast<XRecordRange*>(p); }
}

namespace {

Display* xDisplay() {
    if (auto* x11 = qApp->nativeInterface<QNativeInterface::QX11Application>()) {
        return x11->display();
    }
    return nullptr;
}

// Translate a Qt::Key into an X11 KeyCode by going through KeySym.
KeyCode qtKeyToKeyCode(Display* display, int qtKey) {
    if (!display || !qtKey) {
        return 0;
    }

    QString name;
    switch (qtKey) {
        case Qt::Key_Insert: name = "Insert"; break;
        case Qt::Key_Meta:   name = "Super_L"; break;
        default:
            name = QKeySequence(qtKey).toString();
            break;
    }

    if (name.isEmpty()) {
        return 0;
    }

    KeySym sym = XStringToKeysym(name.toLatin1().constData());
    if (sym == NoSymbol) {
        // Single-character names need to be lowered for X11 keysym lookup.
        sym = XStringToKeysym(name.toLower().toLatin1().constData());
    }
    if (sym == NoSymbol) {
        return 0;
    }
    return XKeysymToKeycode(display, sym);
}

// Local error handler used to detect XGrabKey/XUngrabKey failures (BadAccess
// when another client already has the grab).
class GrabErrorHandler {
public:
    static bool s_error;
    static int handle(Display*, XErrorEvent* event) {
        switch (event->error_code) {
            case BadAccess:
            case BadValue:
            case BadWindow:
                if (event->request_code == 33 /* X_GrabKey */ ||
                    event->request_code == 34 /* X_UngrabKey */) {
                    s_error = true;
                }
        }
        return 0;
    }
    GrabErrorHandler() {
        s_error = false;
        m_prev  = XSetErrorHandler(&handle);
    }
    ~GrabErrorHandler() {
        if (Display* d = xDisplay()) XFlush(d);
        XSetErrorHandler(m_prev);
    }
    bool isError() const {
        if (Display* d = xDisplay()) XFlush(d);
        return s_error;
    }
private:
    int (*m_prev)(Display*, XErrorEvent*) = nullptr;
};
bool GrabErrorHandler::s_error = false;

} // namespace

bool GlobalHotkeyManager::init() {
    // Wayland or other non-xcb session: no-op success path.
    if (QGuiApplication::platformName() != QLatin1String("xcb")) {
        return false;
    }

    Display* display = xDisplay();
    if (!display) {
        return false;
    }

    m_lShiftCode = XKeysymToKeycode(display, XK_Shift_L);
    m_rShiftCode = XKeysymToKeycode(display, XK_Shift_R);
    m_lCtrlCode  = XKeysymToKeycode(display, XK_Control_L);
    m_rCtrlCode  = XKeysymToKeycode(display, XK_Control_R);
    m_lAltCode   = XKeysymToKeycode(display, XK_Alt_L);
    m_rAltCode   = XKeysymToKeycode(display, XK_Alt_R);
    m_lMetaCode  = XKeysymToKeycode(display, XK_Super_L);
    m_rMetaCode  = XKeysymToKeycode(display, XK_Super_R);

    m_cCode        = XKeysymToKeycode(display, XK_c);
    m_insertCode   = XKeysymToKeycode(display, XK_Insert);
    m_kpInsertCode = XKeysymToKeycode(display, XK_KP_Insert);

    m_currentModifiers = 0;
    m_keyToUngrab      = m_grabbedKeys.end();

    // A second display for the XRecord context, since the recording client
    // must not share its connection with the application.
    m_dataDisplay = XOpenDisplay(nullptr);
    if (!m_dataDisplay) {
        return false;
    }

    XRecordRange* range = XRecordAllocRange();
    if (!range) {
        XCloseDisplay(asDisplay(m_dataDisplay));
        m_dataDisplay = nullptr;
        return false;
    }
    range->device_events.first = KeyPress;
    range->device_events.last  = KeyRelease;
    m_recordRange      = range;
    m_recordClientSpec = XRecordAllClients;

    XRecordClientSpec spec = static_cast<XRecordClientSpec>(m_recordClientSpec);
    m_recordContext = XRecordCreateContext(display, 0, &spec, 1, &range, 1);
    if (!m_recordContext) {
        XFree(range);
        m_recordRange = nullptr;
        XCloseDisplay(asDisplay(m_dataDisplay));
        m_dataDisplay = nullptr;
        return false;
    }

    // Ensure the context is created before the worker thread starts.
    XSync(display, False);

    connect(this, &GlobalHotkeyManager::keyRecorded,
            this, &GlobalHotkeyManager::checkState,
            Qt::QueuedConnection);

    start(); // QThread::run() -> XRecordEnableContext loop
    return true;
}

void GlobalHotkeyManager::shutdown() {
    Display* display = xDisplay();
    if (display && m_recordContext) {
        XRecordDisableContext(display, static_cast<XRecordContext>(m_recordContext));
        XSync(display, False);
    }

    wait(); // join the worker thread

    if (display && m_recordContext) {
        XRecordFreeContext(display, static_cast<XRecordContext>(m_recordContext));
        m_recordContext = 0;
    }
    if (m_recordRange) {
        XFree(asRange(m_recordRange));
        m_recordRange = nullptr;
    }
    if (m_dataDisplay) {
        XCloseDisplay(asDisplay(m_dataDisplay));
        m_dataDisplay = nullptr;
    }

    while (!m_grabbedKeys.empty()) {
        ungrabKey(m_grabbedKeys.begin());
    }
}

void GlobalHotkeyManager::run() {
    auto trampoline = [](XPointer ptr, XRecordInterceptData* data) {
        GlobalHotkeyManager::recordEventCallback(reinterpret_cast<void*>(ptr),
                                                 reinterpret_cast<void*>(data));
    };
    if (!XRecordEnableContext(asDisplay(m_dataDisplay),
                              static_cast<XRecordContext>(m_recordContext),
                              trampoline,
                              reinterpret_cast<XPointer>(this))) {
        qWarning("GlobalHotkeyManager: XRecordEnableContext failed");
    }
}

void GlobalHotkeyManager::recordEventCallback(void* ptr, void* data) {
    reinterpret_cast<GlobalHotkeyManager*>(ptr)->handleRecordEvent(data);
}

void GlobalHotkeyManager::handleRecordEvent(void* dataPtr) {
    XRecordInterceptData* data = reinterpret_cast<XRecordInterceptData*>(dataPtr);
    if (data->category == XRecordFromServer) {
        xEvent* event = reinterpret_cast<xEvent*>(data->data);

        if (event->u.u.type == KeyPress) {
            KeyCode key = event->u.u.detail;

            if (key == m_lShiftCode || key == m_rShiftCode) {
                m_currentModifiers |= ShiftMask;
            } else if (key == m_lCtrlCode || key == m_rCtrlCode) {
                m_currentModifiers |= ControlMask;
            } else if (key == m_lAltCode || key == m_rAltCode) {
                m_currentModifiers |= Mod1Mask;
            } else if (key == m_lMetaCode || key == m_rMetaCode) {
                m_currentModifiers |= Mod4Mask;
            } else {
                if (key == m_kpInsertCode) {
                    key = m_insertCode;
                }
                emit keyRecorded(key, m_currentModifiers);
            }
        } else if (event->u.u.type == KeyRelease) {
            KeyCode key = event->u.u.detail;
            if (key == m_lShiftCode || key == m_rShiftCode) {
                m_currentModifiers &= ~ShiftMask;
            } else if (key == m_lCtrlCode || key == m_rCtrlCode) {
                m_currentModifiers &= ~ControlMask;
            } else if (key == m_lAltCode || key == m_rAltCode) {
                m_currentModifiers &= ~Mod1Mask;
            } else if (key == m_lMetaCode || key == m_rMetaCode) {
                m_currentModifiers &= ~Mod4Mask;
            }
        }
    }
    XRecordFreeData(data);
}

bool GlobalHotkeyManager::checkState(quint32 vk, quint32 mod) {
    if (m_state2) {
        waitKey2(); // cancel pending wait

        if (m_state2waiter.key2 == vk && m_state2waiter.modifier == mod) {
            emit hotkeyActivated(m_state2waiter.handle);
            return true;
        }
    }

    for (const HotkeyEntry& hs : m_hotkeys) {
        if (hs.key == vk && hs.modifier == mod) {
            if (hs.key2 == 0) {
                emit hotkeyActivated(hs.handle);
                return true;
            }

            m_state2       = true;
            m_state2waiter = hs;
            QTimer::singleShot(500, this, &GlobalHotkeyManager::waitKey2);

            // Grab the second key only when it isn't itself a copy combo
            // (else we'd block the user's own copy keystroke).
            if ((isCopyToClipboardKey(hs.key, hs.modifier) ||
                 !isCopyToClipboardKey(hs.key2, hs.modifier)) &&
                !isKeyGrabbed(hs.key2, hs.modifier)) {
                m_keyToUngrab = grabKey(hs.key2, hs.modifier);
            }
            return true;
        }
    }

    m_state2 = false;
    return false;
}

quint32 GlobalHotkeyManager::nativeKey(int qtKey) const {
    return qtKeyToKeyCode(xDisplay(), qtKey);
}

bool GlobalHotkeyManager::isCopyToClipboardKey(quint32 keyCode, quint32 modifiers) const {
    return modifiers == ControlMask &&
           (keyCode == m_cCode || keyCode == m_insertCode || keyCode == m_kpInsertCode);
}

bool GlobalHotkeyManager::isKeyGrabbed(quint32 keyCode, quint32 modifiers) const {
    return m_grabbedKeys.find(std::make_pair(keyCode, modifiers)) != m_grabbedKeys.end();
}

GlobalHotkeyManager::GrabbedKeys::iterator
GlobalHotkeyManager::grabKey(quint32 keyCode, quint32 modifiers) {
    auto result = m_grabbedKeys.insert(std::make_pair(keyCode, modifiers));

    if (result.second) {
        Display* display = xDisplay();
        if (!display) {
            return result.first;
        }
        GrabErrorHandler handler;
        XGrabKey(display, keyCode, modifiers, DefaultRootWindow(display),
                 True, GrabModeAsync, GrabModeAsync);

        if (handler.isError()) {
            qWarning("GlobalHotkeyManager: XGrabKey reports a hotkey conflict");
            ungrabKey(result.first);
        }
    }

    return result.first;
}

bool GlobalHotkeyManager::registerHotkey(const QKeySequence& sequence, int handle) {
    if (!m_initialized) {
        return true;
    }

    Qt::KeyboardModifiers modifier;
    int key1 = 0;
    int key2 = 0;
    if (!parseSequence(sequence, modifier, key1, key2)) {
        return false;
    }

    quint32 vk  = nativeKey(key1);
    quint32 vk2 = key2 ? nativeKey(key2) : 0;
    if (!vk) {
        return false;
    }

    quint32 mod = 0;
    if (modifier & Qt::ShiftModifier)   mod |= ShiftMask;
    if (modifier & Qt::ControlModifier) mod |= ControlMask;
    if (modifier & Qt::AltModifier)     mod |= Mod1Mask;
    if (modifier & Qt::MetaModifier)    mod |= Mod4Mask;

    m_hotkeys.append(HotkeyEntry(vk, vk2, mod, handle, 0));

    // Don't grab Ctrl+C globally; intercepting it would block the user's
    // own copy in the foreground app. XRecord observes the press anyway.
    if (!isCopyToClipboardKey(vk, mod)) {
        grabKey(vk, mod);
    }
    return true;
}

void GlobalHotkeyManager::unregisterAll() {
    while (!m_grabbedKeys.empty()) {
        ungrabKey(m_grabbedKeys.begin());
    }
    m_hotkeys.clear();
    m_state2 = false;
}

void GlobalHotkeyManager::ungrabKey(GrabbedKeys::iterator it) {
    Display* display = xDisplay();
    if (display) {
        GrabErrorHandler handler;
        XUngrabKey(display, it->first, it->second, XDefaultRootWindow(display));
        if (handler.isError()) {
            qWarning("GlobalHotkeyManager: XUngrabKey failed");
        }
    }
    m_grabbedKeys.erase(it);
}

#endif // WITH_X11
