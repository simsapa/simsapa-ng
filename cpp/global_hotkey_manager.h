#ifndef GLOBAL_HOTKEY_MANAGER_H
#define GLOBAL_HOTKEY_MANAGER_H

// GlobalHotkeyManager: cross-platform OS-level global hotkey registration
// for Simsapa.
//
// Platform implementations live in separate translation units guarded by
// preprocessor defines:
//   - Linux X11 -> cpp/global_hotkey_x11.cpp     (defined WITH_X11)
//   - Windows   -> cpp/global_hotkey_win.cpp     (Q_OS_WIN)
//   - macOS     -> cpp/global_hotkey_mac.mm      (Q_OS_MACOS)
//
// For Wayland and any other platform the manager falls back to a no-op
// (registerHotkey returns true with no OS-level grab).

#include <QKeySequence>
#include <QList>
#include <QObject>
#include <QString>
#include <QThread>

#ifdef WITH_X11
  // We deliberately do NOT include <X11/Xlib.h> in the header: X11 leaks
  // macros (`Bool`, `min`, `max`, `Status`) that corrupt Qt's MOC output if
  // they reach moc_*.cpp via mocs_compilation.cpp's bulk include. All X11
  // state is held as opaque types here and cast back to its real type
  // inside `cpp/global_hotkey_x11.cpp`.
  #include <set>
  #include <utility>
#endif

struct HotkeyEntry {
    HotkeyEntry() = default;
    HotkeyEntry(quint32 key_, quint32 key2_, quint32 modifier_, int handle_, int id_)
        : key(key_), key2(key2_), modifier(modifier_), handle(handle_), id(id_) {}

    quint32 key      = 0;
    quint32 key2     = 0;
    quint32 modifier = 0;
    int handle       = 0;
    int id           = 0;
};

class GlobalHotkeyManager : public QThread {
    Q_OBJECT

public:
    explicit GlobalHotkeyManager(QObject* parent = nullptr);
    ~GlobalHotkeyManager() override;

    /// Register a key sequence under an integer handle. Returns false if the
    /// sequence is empty or the OS-level registration fails. On platforms
    /// without backend support (Wayland / Android), returns true and is a
    /// no-op (the rest of the app continues to function).
    bool registerHotkey(const QKeySequence& sequence, int handle);

    /// Unregister every hotkey currently held by this manager and release
    /// any OS-level grabs.
    void unregisterAll();

    /// Normalise the user-friendly double-tap form `"Ctrl+C+C"` into Qt's
    /// native chord-separator form `"Ctrl+C, C"` (used by QKeySequence's
    /// multi-chord parser). Single-chord sequences like `"Ctrl+Alt+L"` are
    /// returned unchanged.
    static QString normalizeSequenceString(const QString& s);

    /// Whether platform initialization succeeded. Returns false on a
    /// platform that has no backend (Wayland, Android) or if init() failed
    /// (e.g. missing X11 RECORD extension).
    bool isInitialized() const { return m_initialized; }

    /// Parse a Qt-style key sequence string ("Ctrl+C+C", "Ctrl+Alt+L") into
    /// (modifier, key1, key2). key2 is 0 for single-chord sequences.
    static bool parseSequence(const QKeySequence& seq,
                              Qt::KeyboardModifiers& outModifier,
                              int& outKey1,
                              int& outKey2);

signals:
    void hotkeyActivated(int handle);

#ifdef WITH_X11
    /// Emitted from the XRecord worker thread, queued back to the main
    /// thread (see Qt::QueuedConnection in init()).
    void keyRecorded(quint32 vk, quint32 mod);
#endif

protected slots:
    /// Called via QTimer::singleShot ~500 ms after the first chord of a
    /// double-tap sequence: clears the "waiting for second key" state.
    void waitKey2();

#ifdef WITH_X11
private slots:
    bool checkState(quint32 vk, quint32 mod);
#endif

protected:
#ifdef WITH_X11
    /// QThread entry point -- runs XRecordEnableContext (blocking).
    void run() override;
#endif

private:
    bool init();
    void shutdown();
    quint32 nativeKey(int qtKey) const;

    QList<HotkeyEntry> m_hotkeys;
    bool m_state2 = false;
    HotkeyEntry m_state2waiter;
    bool m_initialized = false;

#ifdef WITH_X11
    // Raw X11 types kept opaque in the header (see note above):
    //   * KeyCode       -> quint32
    //   * Display*      -> void*
    //   * XRecordRange* -> void*
    //   * XRecordContext / XRecordClientSpec -> unsigned long (XID typedef)
    //   * XPointer / XRecordInterceptData* -> void*
    static void recordEventCallback(void* ptr, void* data);
    void handleRecordEvent(void* data);

    quint32 m_lShiftCode = 0, m_rShiftCode = 0;
    quint32 m_lCtrlCode = 0,  m_rCtrlCode = 0;
    quint32 m_lAltCode = 0,   m_rAltCode = 0;
    quint32 m_lMetaCode = 0,  m_rMetaCode = 0;
    quint32 m_cCode = 0, m_insertCode = 0, m_kpInsertCode = 0;

    quint32 m_currentModifiers = 0;

    void*         m_dataDisplay      = nullptr; // Display*
    void*         m_recordRange      = nullptr; // XRecordRange*
    unsigned long m_recordContext    = 0;       // XRecordContext
    unsigned long m_recordClientSpec = 0;       // XRecordClientSpec

    using GrabbedKeys = std::set<std::pair<quint32, quint32>>;
    GrabbedKeys m_grabbedKeys;
    GrabbedKeys::iterator m_keyToUngrab;

    bool isCopyToClipboardKey(quint32 keyCode, quint32 modifiers) const;
    bool isKeyGrabbed(quint32 keyCode, quint32 modifiers) const;
    GrabbedKeys::iterator grabKey(quint32 keyCode, quint32 modifiers);
    void ungrabKey(GrabbedKeys::iterator it);
#endif
};

#endif // GLOBAL_HOTKEY_MANAGER_H
