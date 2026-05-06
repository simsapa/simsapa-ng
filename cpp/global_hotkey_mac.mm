// macOS backend for GlobalHotkeyManager.
//
// Uses Carbon's RegisterEventHotKey to grab the configured chord globally
// (the Carbon hotkey API is the community-recommended path for global
// hotkeys on macOS even though Carbon is otherwise deprecated; see the
// KeyboardShortcuts library README cited in Goldendict-ng).
//
// For the Cmd+C+C "scan selection" flow, the first Cmd+C is captured by us,
// which prevents the foreground app from performing its own copy. To recover
// the user's selection we:
//   1. Try the Accessibility API (AXUIElementCopyAttributeValue with
//      kAXSelectedTextAttribute) on the system-wide focused element. This is
//      instant and doesn't pollute the clipboard.
//   2. If that returns empty, suspend our hotkey grabs, synthesize a Cmd+C
//      via CGEventCreateKeyboardEvent / CGEventPost, then resume them. The
//      foreground app sees a fresh Cmd+C and copies the selection to the
//      clipboard normally.
//
// Both of those paths require the user to have granted Accessibility
// permission to Simsapa. On first activation we check AXIsProcessTrusted()
// and, if false, show a one-time dialog with a button that opens the
// Accessibility pane in System Preferences.
//
// Design reference (GPLv3): Goldendict-ng's `machotkeywrapper.mm`. No source
// files are copied verbatim; this is Simsapa's own implementation.

#ifdef Q_OS_MACOS

#include "global_hotkey_manager.h"

#include <QClipboard>
#include <QGuiApplication>
#include <QMessageBox>
#include <QPushButton>
#include <QString>
#include <QTimer>

#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <Carbon/Carbon.h>

#include <vector>

namespace {

// Build a UniChar -> native keycode map once at startup using the current
// keyboard layout. For Latin-printable characters the inverse of UCKeyTranslate
// gives us the native virtual key code expected by RegisterEventHotKey.
struct ReverseMapEntry {
    UniChar character;
    UInt16 keyCode;
};

std::vector<ReverseMapEntry> g_keyMap;

void buildKeyMap() {
    if (!g_keyMap.empty()) {
        return;
    }
    g_keyMap.reserve(128);

    TISInputSourceRef src = TISCopyCurrentKeyboardLayoutInputSource();
    if (!src) {
        return;
    }
    CFDataRef uchrData =
        (CFDataRef)TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData);
    const UCKeyboardLayout* layout = nullptr;
    if (uchrData) {
        layout = reinterpret_cast<const UCKeyboardLayout*>(CFDataGetBytePtr(uchrData));
    }
    if (!layout) {
        CFRelease(src);
        return;
    }

    for (UInt16 vk = 0; vk < 128; ++vk) {
        UInt32 deadKeyState = 0;
        UniCharCount len = 0;
        UniChar buf = 0;
        OSStatus s = UCKeyTranslate(layout, vk, kUCKeyActionDown, 0,
                                    LMGetKbdType(), kUCKeyTranslateNoDeadKeysBit,
                                    &deadKeyState, 1, &len, &buf);
        if (s == noErr && len > 0 && std::isprint(buf)) {
            g_keyMap.push_back({buf, vk});
        }
    }
    CFRelease(src);
}

quint32 lookupNativeForChar(UniChar ch) {
    buildKeyMap();
    for (const auto& e : g_keyMap) {
        if (e.character == ch) {
            return e.keyCode;
        }
    }
    return 0;
}

quint32 qtKeyToMacKey(int key) {
    switch (key) {
        case Qt::Key_Escape:    return 0x35;
        case Qt::Key_Tab:       return 0x30;
        case Qt::Key_Backtab:   return 0x30;
        case Qt::Key_Backspace: return 0x33;
        case Qt::Key_Return:    return 0x24;
        case Qt::Key_Enter:     return 0x4c;
        case Qt::Key_Delete:    return 0x75;
        case Qt::Key_Clear:     return 0x47;
        case Qt::Key_Home:      return 0x73;
        case Qt::Key_End:       return 0x77;
        case Qt::Key_Left:      return 0x7b;
        case Qt::Key_Up:        return 0x7e;
        case Qt::Key_Right:     return 0x7c;
        case Qt::Key_Down:      return 0x7d;
        case Qt::Key_PageUp:    return 0x74;
        case Qt::Key_PageDown:  return 0x79;
        case Qt::Key_CapsLock:  return 0x57;
        case Qt::Key_F1:        return 0x7a;
        case Qt::Key_F2:        return 0x78;
        case Qt::Key_F3:        return 0x63;
        case Qt::Key_F4:        return 0x76;
        case Qt::Key_F5:        return 0x60;
        case Qt::Key_F6:        return 0x61;
        case Qt::Key_F7:        return 0x62;
        case Qt::Key_F8:        return 0x64;
        case Qt::Key_F9:        return 0x65;
        case Qt::Key_F10:       return 0x6d;
        case Qt::Key_F11:       return 0x67;
        case Qt::Key_F12:       return 0x6f;
        case Qt::Key_F13:       return 0x69;
        case Qt::Key_F14:       return 0x6b;
        case Qt::Key_F15:       return 0x71;
        case Qt::Key_Help:      return 0x72;
        case Qt::Key_Space:     return 0x31;
        default: break;
    }
    return lookupNativeForChar(QChar(key).toLower().unicode());
}

QString getSelectedTextViaAxApi() {
    QString result;
    AXUIElementRef systemWide = AXUIElementCreateSystemWide();
    if (!systemWide) {
        return result;
    }

    AXUIElementRef focused = nullptr;
    AXError err = AXUIElementCopyAttributeValue(
        systemWide, kAXFocusedUIElementAttribute, (CFTypeRef*)&focused);
    if (err == kAXErrorSuccess && focused) {
        CFTypeRef selected = nullptr;
        err = AXUIElementCopyAttributeValue(
            focused, kAXSelectedTextAttribute, (CFTypeRef*)&selected);
        if (err == kAXErrorSuccess && selected) {
            if (CFGetTypeID(selected) == CFStringGetTypeID()) {
                result = QString::fromCFString((CFStringRef)selected);
            }
            CFRelease(selected);
        }
        CFRelease(focused);
    }
    CFRelease(systemWide);
    return result;
}

OSStatus hotKeyHandler(EventHandlerCallRef /*nextHandler*/,
                       EventRef event, void* userData) {
    EventHotKeyID hkId{};
    GetEventParameter(event, kEventParamDirectObject, typeEventHotKeyID, nullptr,
                      sizeof(EventHotKeyID), nullptr, &hkId);
    auto* mgr = static_cast<GlobalHotkeyManager*>(userData);
    if (mgr) {
        mgr->checkStateMac(static_cast<int>(hkId.id));
    }
    return noErr;
}

} // namespace

bool GlobalHotkeyManager::init() {
    EventTypeSpec evt;
    evt.eventClass = kEventClassKeyboard;
    evt.eventKind  = kEventHotKeyPressed;

    EventHandlerUPP upp = NewEventHandlerUPP(hotKeyHandler);
    EventHandlerRef ref = nullptr;
    OSStatus s = InstallApplicationEventHandler(upp, 1, &evt, this, &ref);
    if (s != noErr) {
        DisposeEventHandlerUPP(upp);
        return false;
    }

    m_macHandlerUPP = reinterpret_cast<void*>(upp);
    m_macHandlerRef = reinterpret_cast<void*>(ref);
    m_macKeyC       = lookupNativeForChar('c');
    return true;
}

void GlobalHotkeyManager::shutdown() {
    unregisterAll();
    if (m_macHandlerRef) {
        RemoveEventHandler(reinterpret_cast<EventHandlerRef>(m_macHandlerRef));
        m_macHandlerRef = nullptr;
    }
    if (m_macHandlerUPP) {
        DisposeEventHandlerUPP(reinterpret_cast<EventHandlerUPP>(m_macHandlerUPP));
        m_macHandlerUPP = nullptr;
    }
}

quint32 GlobalHotkeyManager::nativeKey(int qtKey) const {
    return qtKeyToMacKey(qtKey);
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
    // Qt::CTRL maps to Cmd on macOS by Qt convention; honour that here so the
    // "Ctrl+C+C" default actually means "Cmd+C+C" for Mac users.
    if (modifier & Qt::ControlModifier) mod |= cmdKey;
    if (modifier & Qt::AltModifier)     mod |= optionKey;
    if (modifier & Qt::ShiftModifier)   mod |= shiftKey;
    if (modifier & Qt::MetaModifier)    mod |= controlKey;

    if (m_macNextId > 0xBFFF - 1) {
        m_macNextId = 1;
    }
    const int firstId = m_macNextId;
    m_macNextId += 2;

    HotkeyEntry entry(vk, vk2, mod, handle, firstId);

    EventHotKeyID hkId{};
    hkId.signature = 'SMSP'; // 'Simsapa'
    hkId.id        = static_cast<UInt32>(firstId);

    EventHotKeyRef ref1 = nullptr;
    OSStatus s = RegisterEventHotKey(vk, mod, hkId, GetApplicationEventTarget(), 0, &ref1);
    if (s != noErr) {
        return false;
    }
    entry.hkRef = reinterpret_cast<void*>(ref1);

    if (vk2 && vk2 != vk) {
        hkId.id = static_cast<UInt32>(firstId + 1);
        EventHotKeyRef ref2 = nullptr;
        s = RegisterEventHotKey(vk2, mod, hkId, GetApplicationEventTarget(), 0, &ref2);
        if (s != noErr) {
            UnregisterEventHotKey(ref1);
            return false;
        }
        entry.hkRef2 = reinterpret_cast<void*>(ref2);
    }

    m_hotkeys.append(entry);
    return true;
}

void GlobalHotkeyManager::unregisterAll() {
    for (HotkeyEntry& e : m_hotkeys) {
        if (e.hkRef) {
            UnregisterEventHotKey(reinterpret_cast<EventHotKeyRef>(e.hkRef));
            e.hkRef = nullptr;
        }
        if (e.hkRef2) {
            UnregisterEventHotKey(reinterpret_cast<EventHotKeyRef>(e.hkRef2));
            e.hkRef2 = nullptr;
        }
    }
    m_hotkeys.clear();
    m_state2 = false;
}

void GlobalHotkeyManager::suspendHotkeysMac() {
    for (HotkeyEntry& e : m_hotkeys) {
        if (e.hkRef) {
            UnregisterEventHotKey(reinterpret_cast<EventHotKeyRef>(e.hkRef));
            e.hkRef = nullptr;
        }
        if (e.hkRef2) {
            UnregisterEventHotKey(reinterpret_cast<EventHotKeyRef>(e.hkRef2));
            e.hkRef2 = nullptr;
        }
    }
}

void GlobalHotkeyManager::resumeHotkeysMac() {
    for (HotkeyEntry& e : m_hotkeys) {
        EventHotKeyID hkId{};
        hkId.signature = 'SMSP';
        hkId.id        = static_cast<UInt32>(e.id);

        EventHotKeyRef ref1 = nullptr;
        if (RegisterEventHotKey(e.key, e.modifier, hkId,
                                GetApplicationEventTarget(), 0, &ref1) == noErr) {
            e.hkRef = reinterpret_cast<void*>(ref1);
        }

        if (e.key2 && e.key2 != e.key) {
            hkId.id = static_cast<UInt32>(e.id + 1);
            EventHotKeyRef ref2 = nullptr;
            if (RegisterEventHotKey(e.key2, e.modifier, hkId,
                                    GetApplicationEventTarget(), 0, &ref2) == noErr) {
                e.hkRef2 = reinterpret_cast<void*>(ref2);
            }
        }
    }
}

void GlobalHotkeyManager::sendCmdC() {
    if (!m_macKeyC) {
        return;
    }
    CGEventSourceRef source = CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
    const CGEventFlags cmd = kCGEventFlagMaskCommand;

    CGEventRef down = CGEventCreateKeyboardEvent(source, m_macKeyC, true);
    CGEventSetFlags(down, CGEventFlags(cmd | CGEventGetFlags(down)));
    CGEventPost(kCGAnnotatedSessionEventTap, down);
    CFRelease(down);

    CGEventRef up = CGEventCreateKeyboardEvent(source, m_macKeyC, false);
    CGEventSetFlags(up, CGEventFlags(cmd | CGEventGetFlags(up)));
    CGEventPost(kCGAnnotatedSessionEventTap, up);
    CFRelease(up);

    if (source) {
        CFRelease(source);
    }
}

void GlobalHotkeyManager::checkAndRequestAccessibilityPermission() {
    if (AXIsProcessTrusted()) {
        return;
    }
    if (m_macAxPromptShown) {
        return;
    }
    m_macAxPromptShown = true;

    QMessageBox box(nullptr);
    box.setIcon(QMessageBox::Information);
    box.setWindowTitle(QStringLiteral("Accessibility permission required"));
    box.setText(QStringLiteral(
        "Simsapa's global Cmd+C+C dictionary lookup needs Accessibility "
        "permission to read the selected text from the focused application."));
    box.setInformativeText(QStringLiteral(
        "Open System Settings → Privacy & Security → Accessibility and enable "
        "Simsapa, then try the hotkey again."));

    auto* openBtn = box.addButton(QStringLiteral("Open Accessibility Settings"),
                                  QMessageBox::AcceptRole);
    box.addButton(QMessageBox::Cancel);
    box.setDefaultButton(openBtn);
    box.exec();

    if (box.clickedButton() == openBtn) {
        NSURL* url = [NSURL URLWithString:
            @"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"];
        [[NSWorkspace sharedWorkspace] openURL:url];
    }
}

void GlobalHotkeyManager::checkStateMac(int hkId) {
    // Awaiting the second chord of a double-tap?
    if (m_state2) {
        waitKey2(); // cancel pending wait

        const bool sameRepeat =
            (hkId == m_state2waiter.id && m_state2waiter.key == m_state2waiter.key2);
        if (hkId == m_state2waiter.id + 1 || sameRepeat) {
            emit hotkeyActivated(m_state2waiter.handle);
            return;
        }
        // Fall through: the press might still match an unrelated hotkey.
    }

    for (int i = 0; i < m_hotkeys.size(); ++i) {
        const HotkeyEntry& hs = m_hotkeys.at(i);
        if (hkId != hs.id) {
            continue;
        }

        // First chord: if it's Cmd+C, capture the user's selection now.
        if (hs.key == m_macKeyC && hs.modifier == cmdKey) {
            checkAndRequestAccessibilityPermission();

            QString text = getSelectedTextViaAxApi();
            if (!text.isEmpty()) {
                QGuiApplication::clipboard()->setText(text);
            } else {
                // Re-emit Cmd+C so the foreground app performs its own copy.
                // Suspend our grabs first so we don't trigger ourselves.
                suspendHotkeysMac();
                sendCmdC();
                resumeHotkeysMac();
            }
        }

        if (hs.key2 == 0) {
            emit hotkeyActivated(hs.handle);
            return;
        }

        m_state2       = true;
        m_state2waiter = hs;
        QTimer::singleShot(500, this, &GlobalHotkeyManager::waitKey2);
        return;
    }

    m_state2 = false;
}

#endif // Q_OS_MACOS
