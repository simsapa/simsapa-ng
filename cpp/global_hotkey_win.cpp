// Windows backend for GlobalHotkeyManager.
//
// Uses RegisterHotKey / UnregisterHotKey to grab the configured chord
// globally. Because RegisterHotKey *consumes* the keystroke, when the first
// chord is itself a copy combo (e.g. Ctrl+C as the first half of Ctrl+C+C),
// the manager re-emits the keystroke via SendInput so the foreground app
// performs its own copy. The hotkey is briefly UnregisterHotKey'd around the
// SendInput call so we don't re-trigger ourselves.
//
// Design reference (GPLv3): Goldendict-ng's `winhotkeywrapper.cc` — the
// re-emit pattern, the GetAsyncKeyState modifier-state checks, and the
// double-tap state2 timer all follow that proven approach. No source files
// are copied verbatim; this is Simsapa's own implementation.

#ifdef Q_OS_WIN

#include "global_hotkey_manager.h"

#include <QAbstractNativeEventFilter>
#include <QCoreApplication>
#include <QTimer>

#include <windows.h>

namespace {

// Forwards every WM_HOTKEY message to the manager. Lives for the lifetime of
// the manager and is installed on QCoreApplication::instance().
class WinHotkeyEventFilter : public QAbstractNativeEventFilter {
public:
    explicit WinHotkeyEventFilter(GlobalHotkeyManager* mgr) : m_mgr(mgr) {}

    bool nativeEventFilter(const QByteArray& /*eventType*/,
                           void* message,
                           qintptr* /*result*/) override {
        MSG* msg = reinterpret_cast<MSG*>(message);
        if (msg && msg->message == WM_HOTKEY) {
            // Per MSDN: HIWORD(lParam) = virtual-key code,
            //           LOWORD(lParam) = modifiers (MOD_*).
            const quint32 vk  = static_cast<quint32>((msg->lParam >> 16) & 0xFFFF);
            const quint32 mod = static_cast<quint32>(msg->lParam & 0xFFFF);
            return m_mgr->checkStateWin(vk, mod);
        }
        return false;
    }

private:
    GlobalHotkeyManager* m_mgr;
};

// Map a Qt::Key to the Windows virtual-key code expected by RegisterHotKey.
// References:
//   https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes
//   https://doc.qt.io/qt-6/qt.html#Key-enum
quint32 qtKeyToVk(int key) {
    // Qt::Key_0..9 and Qt::Key_A..Z map directly onto Windows VK codes.
    if ((key >= Qt::Key_0 && key <= Qt::Key_9) ||
        (key >= Qt::Key_A && key <= Qt::Key_Z)) {
        return static_cast<quint32>(key);
    }

    switch (key) {
        case Qt::Key_Space:       return VK_SPACE;
        case Qt::Key_Tab:
        case Qt::Key_Backtab:     return VK_TAB;
        case Qt::Key_Backspace:   return VK_BACK;
        case Qt::Key_Return:
        case Qt::Key_Enter:       return VK_RETURN;
        case Qt::Key_Escape:      return VK_ESCAPE;
        case Qt::Key_Insert:      return VK_INSERT;
        case Qt::Key_Delete:      return VK_DELETE;
        case Qt::Key_Pause:       return VK_PAUSE;
        case Qt::Key_Print:       return VK_PRINT;
        case Qt::Key_Clear:       return VK_CLEAR;
        case Qt::Key_Home:        return VK_HOME;
        case Qt::Key_End:         return VK_END;
        case Qt::Key_Up:          return VK_UP;
        case Qt::Key_Down:        return VK_DOWN;
        case Qt::Key_Left:        return VK_LEFT;
        case Qt::Key_Right:       return VK_RIGHT;
        case Qt::Key_PageUp:      return VK_PRIOR;
        case Qt::Key_PageDown:    return VK_NEXT;
        case Qt::Key_F1:          return VK_F1;
        case Qt::Key_F2:          return VK_F2;
        case Qt::Key_F3:          return VK_F3;
        case Qt::Key_F4:          return VK_F4;
        case Qt::Key_F5:          return VK_F5;
        case Qt::Key_F6:          return VK_F6;
        case Qt::Key_F7:          return VK_F7;
        case Qt::Key_F8:          return VK_F8;
        case Qt::Key_F9:          return VK_F9;
        case Qt::Key_F10:         return VK_F10;
        case Qt::Key_F11:         return VK_F11;
        case Qt::Key_F12:         return VK_F12;
        case Qt::Key_F13:         return VK_F13;
        case Qt::Key_F14:         return VK_F14;
        case Qt::Key_F15:         return VK_F15;
        case Qt::Key_F16:         return VK_F16;
        case Qt::Key_F17:         return VK_F17;
        case Qt::Key_F18:         return VK_F18;
        case Qt::Key_F19:         return VK_F19;
        case Qt::Key_F20:         return VK_F20;
        case Qt::Key_F21:         return VK_F21;
        case Qt::Key_F22:         return VK_F22;
        case Qt::Key_F23:         return VK_F23;
        case Qt::Key_F24:         return VK_F24;
        case Qt::Key_Asterisk:    return VK_MULTIPLY;
        case Qt::Key_Plus:        return VK_ADD;
        case Qt::Key_Minus:       return VK_SUBTRACT;
        case Qt::Key_Slash:       return VK_DIVIDE;
        case Qt::Key_Comma:       return VK_OEM_COMMA;
        case Qt::Key_Period:      return VK_OEM_PERIOD;
        case Qt::Key_Equal:       return VK_OEM_PLUS;
        case Qt::Key_Semicolon:
        case Qt::Key_Colon:       return VK_OEM_1;
        case Qt::Key_Question:    return VK_OEM_2;
        case Qt::Key_QuoteLeft:
        case Qt::Key_AsciiTilde:  return VK_OEM_3;
        case Qt::Key_BracketLeft:
        case Qt::Key_BraceLeft:   return VK_OEM_4;
        case Qt::Key_Backslash:
        case Qt::Key_Bar:         return VK_OEM_5;
        case Qt::Key_BracketRight:
        case Qt::Key_BraceRight:  return VK_OEM_6;
        case Qt::Key_Apostrophe:
        case Qt::Key_QuoteDbl:    return VK_OEM_7;
        case Qt::Key_Meta:        return VK_LWIN;
        default: break;
    }

    return static_cast<quint32>(key);
}

// Synthesize the keystroke for the given (vk, mod) so the foreground app
// receives it. Mirrors goldendict's GetAsyncKeyState dance: only emit
// modifier press/release events for modifiers that aren't already physically
// held, otherwise the OS sees a duplicated press.
void reEmitKeystroke(quint32 vk, quint32 mod) {
    INPUT events[10];
    ZeroMemory(events, sizeof(events));
    int count          = 0;
    short pressedMods  = 0;

    auto pushDown = [&](WORD wVk) {
        events[count].type     = INPUT_KEYBOARD;
        events[count].ki.wVk   = wVk;
        ++count;
    };
    auto pushUp = [&](WORD wVk) {
        events[count].type        = INPUT_KEYBOARD;
        events[count].ki.wVk      = wVk;
        events[count].ki.dwFlags  = KEYEVENTF_KEYUP;
        ++count;
    };

    if ((mod & MOD_ALT)     && (GetAsyncKeyState(VK_MENU)    & 0x8000) == 0) {
        pressedMods |= MOD_ALT;
        pushDown(VK_MENU);
    }
    if ((mod & MOD_CONTROL) && (GetAsyncKeyState(VK_CONTROL) & 0x8000) == 0) {
        pressedMods |= MOD_CONTROL;
        pushDown(VK_CONTROL);
    }
    if ((mod & MOD_SHIFT)   && (GetAsyncKeyState(VK_SHIFT)   & 0x8000) == 0) {
        pressedMods |= MOD_SHIFT;
        pushDown(VK_SHIFT);
    }
    if ((mod & MOD_WIN) &&
        (GetAsyncKeyState(VK_LWIN) & 0x8000) == 0 &&
        (GetAsyncKeyState(VK_RWIN) & 0x8000) == 0) {
        pressedMods |= MOD_WIN;
        pushDown(VK_LWIN);
    }

    pushDown(static_cast<WORD>(vk));
    pushUp(static_cast<WORD>(vk));

    if (pressedMods & MOD_WIN)     pushUp(VK_LWIN);
    if (pressedMods & MOD_SHIFT)   pushUp(VK_SHIFT);
    if (pressedMods & MOD_CONTROL) pushUp(VK_CONTROL);
    if (pressedMods & MOD_ALT)     pushUp(VK_MENU);

    SendInput(static_cast<UINT>(count), events, sizeof(INPUT));
}

} // namespace

bool GlobalHotkeyManager::init() {
    auto* filter = new WinHotkeyEventFilter(this);
    m_winFilter  = filter;
    m_winNextId  = 0;
    if (auto* app = QCoreApplication::instance()) {
        app->installNativeEventFilter(filter);
    }
    return true;
}

void GlobalHotkeyManager::shutdown() {
    unregisterAll();
    if (m_winFilter) {
        auto* filter = reinterpret_cast<WinHotkeyEventFilter*>(m_winFilter);
        if (auto* app = QCoreApplication::instance()) {
            app->removeNativeEventFilter(filter);
        }
        delete filter;
        m_winFilter = nullptr;
    }
}

quint32 GlobalHotkeyManager::nativeKey(int qtKey) const {
    return qtKeyToVk(qtKey);
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
    if (modifier & Qt::ShiftModifier)   mod |= MOD_SHIFT;
    if (modifier & Qt::ControlModifier) mod |= MOD_CONTROL;
    if (modifier & Qt::AltModifier)     mod |= MOD_ALT;
    if (modifier & Qt::MetaModifier)    mod |= MOD_WIN;

    // Wrap around defensively so we never collide with the system-reserved
    // global atom range (0xC000-0xFFFF) used by GlobalAddAtom.
    if (m_winNextId > 0xBFFF - 1) {
        m_winNextId = 0;
    }

    const int firstId = m_winNextId++;
    HotkeyEntry entry(vk, vk2, mod, handle, firstId);
    m_hotkeys.append(entry);

    if (!RegisterHotKey(nullptr, firstId, mod, vk)) {
        m_hotkeys.removeLast();
        return false;
    }

    if (vk2 && vk2 != vk) {
        const int secondId = m_winNextId++;
        if (!RegisterHotKey(nullptr, secondId, mod, vk2)) {
            UnregisterHotKey(nullptr, firstId);
            m_hotkeys.removeLast();
            return false;
        }
    }

    return true;
}

void GlobalHotkeyManager::unregisterAll() {
    for (const HotkeyEntry& hs : m_hotkeys) {
        UnregisterHotKey(nullptr, hs.id);
        if (hs.key2 && hs.key2 != hs.key) {
            UnregisterHotKey(nullptr, hs.id + 1);
        }
    }
    m_hotkeys.clear();
    m_state2 = false;
}

bool GlobalHotkeyManager::checkStateWin(quint32 vk, quint32 mod) {
    // Awaiting the second chord of a double-tap?
    if (m_state2) {
        waitKey2(); // cancel pending wait
        if (m_state2waiter.key2 == vk && m_state2waiter.modifier == mod) {
            emit hotkeyActivated(m_state2waiter.handle);
            return true;
        }
        // Fall through: the press might still match an unrelated hotkey.
    }

    for (int i = 0; i < m_hotkeys.size(); ++i) {
        const HotkeyEntry& hs = m_hotkeys.at(i);
        if (hs.key != vk || hs.modifier != mod) {
            continue;
        }

        // Re-emit the keystroke so the foreground application sees it. We do
        // this when:
        //   * this is the first chord of a double-tap (hs.key2 != 0), so
        //     the foreground app's own copy still happens for "Ctrl+C+C", or
        //   * the chord itself is a copy combo (Ctrl+C / Ctrl+Insert), so
        //     the user's plain Ctrl+C still works while a global hotkey
        //     happens to share the same chord.
        const bool isCopyCombo = (mod == MOD_CONTROL) &&
                                 (vk == 'C' || vk == 'c' || vk == VK_INSERT);
        if (hs.key2 != 0 || isCopyCombo) {
            UnregisterHotKey(nullptr, hs.id);
            reEmitKeystroke(vk, mod);
            RegisterHotKey(nullptr, hs.id, hs.modifier, hs.key);
        }

        if (hs.key2 == 0) {
            emit hotkeyActivated(hs.handle);
            return true;
        }

        // Begin double-tap wait window.
        m_state2       = true;
        m_state2waiter = hs;
        QTimer::singleShot(500, this, &GlobalHotkeyManager::waitKey2);
        return true;
    }

    m_state2 = false;
    return false;
}

#endif // Q_OS_WIN
