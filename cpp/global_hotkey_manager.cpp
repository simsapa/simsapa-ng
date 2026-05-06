// Cross-platform shared logic for GlobalHotkeyManager.
//
// Platform-specific behaviour (XRecord, RegisterHotKey, RegisterEventHotKey)
// lives in global_hotkey_x11.cpp / global_hotkey_win.cpp / global_hotkey_mac.mm.
// The double-tap state machine and QKeySequence parsing live here.

#include "global_hotkey_manager.h"

#include <QGuiApplication>
#include <QSet>
#include <QStringList>
#include <QTimer>

GlobalHotkeyManager::GlobalHotkeyManager(QObject* parent)
    : QThread(parent), m_state2(false) {
#ifdef WITH_X11
    m_keyToUngrab = m_grabbedKeys.end();
#endif

    // Best-effort init. Failures (e.g. missing RECORD extension on X11) are
    // logged inside init(); on unsupported platforms init() is a no-op that
    // returns false but the manager still functions as an inert object.
    m_initialized = init();
}

GlobalHotkeyManager::~GlobalHotkeyManager() {
    if (m_initialized) {
        shutdown();
    }
}

void GlobalHotkeyManager::waitKey2() {
    m_state2 = false;

#ifdef WITH_X11
    if (m_keyToUngrab != m_grabbedKeys.end()) {
        ungrabKey(m_keyToUngrab);
        m_keyToUngrab = m_grabbedKeys.end();
    }
#endif
}

QString GlobalHotkeyManager::normalizeSequenceString(const QString& s) {
    // Already in Qt's chord-separator form, leave it.
    if (s.contains(',')) {
        return s;
    }
    const QStringList parts = s.split('+', Qt::SkipEmptyParts);
    if (parts.size() < 3) {
        return s; // single chord like "Ctrl+L" or just "C"
    }

    static const QSet<QString> kModifiers = {
        "Ctrl",    "Control", "Shift", "Alt",    "Meta",
        "Super",   "Cmd",     "Command", "Option",
    };

    // Count non-modifier tokens. A double-tap form has exactly two
    // non-modifier tokens at the tail; a single chord has exactly one.
    int nonModifierCount = 0;
    for (const QString& p : parts) {
        if (!kModifiers.contains(p)) {
            ++nonModifierCount;
        }
    }
    if (nonModifierCount < 2) {
        return s;
    }

    // The trailing token is the double-tap key; everything before it is the
    // first chord. Insert ", " between them.
    QStringList head = parts.mid(0, parts.size() - 1);
    const QString tail = parts.last();
    return head.join('+') + ", " + tail;
}

bool GlobalHotkeyManager::parseSequence(const QKeySequence& seq,
                                        Qt::KeyboardModifiers& outModifier,
                                        int& outKey1,
                                        int& outKey2) {
    outModifier = Qt::NoModifier;
    outKey1     = 0;
    outKey2     = 0;

    // Two supported textual forms:
    //   single chord:  "Ctrl+Alt+L"  -> count() == 1, seq[0] = Ctrl|Alt|Key_L
    //   double-tap:    "Ctrl+C, C"   -> count() == 2 (Qt's chord separator)
    //                  "Ctrl+C+C"    -> ambiguous; we parse it ourselves
    //
    // Simsapa's settings use the "Mod+K+K" form for double-tap, so we parse the
    // string textually if Qt didn't split it into two chords.

    if (seq.count() >= 1) {
        int firstKey = seq[0].toCombined();
        outModifier  = Qt::KeyboardModifiers(firstKey & Qt::KeyboardModifierMask);
        outKey1      = firstKey & ~Qt::KeyboardModifierMask;
    }

    if (seq.count() >= 2) {
        int secondKey = seq[1].toCombined();
        outKey2       = secondKey & ~Qt::KeyboardModifierMask;
    } else {
        // Try to interpret a "Mod+K+K" string where the trailing token is
        // the same key repeated without modifiers.
        const QString text = seq.toString(QKeySequence::PortableText);
        const QStringList parts = text.split('+', Qt::SkipEmptyParts);
        if (parts.size() >= 3) {
            const QString tail = parts.last().trimmed();
            QKeySequence tailSeq(tail);
            if (!tailSeq.isEmpty()) {
                outKey2 = tailSeq[0].toCombined() & ~Qt::KeyboardModifierMask;
                if (outKey1 == 0 && parts.size() >= 2) {
                    // "Ctrl+C+C" can become a single chord with key1 already
                    // set; if not, recover key1 from the second-to-last
                    // token.
                    QKeySequence midSeq(parts.at(parts.size() - 2));
                    if (!midSeq.isEmpty()) {
                        int mid = midSeq[0].toCombined();
                        if (outModifier == Qt::NoModifier) {
                            outModifier = Qt::KeyboardModifiers(mid & Qt::KeyboardModifierMask);
                        }
                        outKey1 = mid & ~Qt::KeyboardModifierMask;
                    }
                }
            }
        }
    }

    return outKey1 != 0;
}

#if !defined(WITH_X11) && !defined(Q_OS_WIN) && !defined(Q_OS_MACOS)

// Stub backend used on Wayland, Android, iOS, and any other platform without
// a dedicated implementation. init() reports failure so registerHotkey()
// short-circuits to a benign no-op (true).

bool GlobalHotkeyManager::init() {
    return false;
}

void GlobalHotkeyManager::shutdown() {}

bool GlobalHotkeyManager::registerHotkey(const QKeySequence& sequence, int handle) {
    Q_UNUSED(sequence);
    Q_UNUSED(handle);
    return true;
}

void GlobalHotkeyManager::unregisterAll() {
    m_hotkeys.clear();
    m_state2 = false;
}

quint32 GlobalHotkeyManager::nativeKey(int qtKey) const {
    Q_UNUSED(qtKey);
    return 0;
}

#endif
