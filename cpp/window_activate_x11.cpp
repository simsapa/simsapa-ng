// X11 activation helpers.
//
// Qt6 removed the QX11Info::setAppUserTime() entry point that Qt5 used to
// stamp _NET_WM_USER_TIME on outgoing activation requests. The public
// QNativeInterface::QX11Application has no equivalent, so we set the property
// directly on the target window. EWMH (§5.2.5) requires the timestamp to be
// the X server time of the user action that triggered the activation; window
// managers (Mutter, KWin, ...) use it to allow or deny focus-stealing.
//
// Lives in its own translation unit because <X11/Xlib.h> leaks the macros
// `Bool`, `min`, `max`, `Status` etc. that corrupt Qt MOC output if dragged
// into window_manager.cpp.

#ifdef WITH_X11

#include <QGuiApplication>

#include <cstring>

#include <X11/Xlib.h>
#include <X11/Xatom.h>

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

extern "C" void x11_set_user_time(unsigned long winid, unsigned int time) {
    if (winid == 0) return;
    auto* x11 = qApp->nativeInterface<QNativeInterface::QX11Application>();
    if (!x11) return;
    Display* d = x11->display();
    if (!d) return;
    Atom user_time_atom = XInternAtom(d, "_NET_WM_USER_TIME", False);
    XChangeProperty(d, static_cast<Window>(winid), user_time_atom,
                    XA_CARDINAL, 32, PropModeReplace,
                    reinterpret_cast<const unsigned char*>(&time), 1);
    XFlush(d);
}

// Send _NET_ACTIVE_WINDOW directly (EWMH §5.2.9). Qt's QWindow::requestActivate
// also sends this message but uses its internal QXcbConnection time tracker,
// which is only updated by events delivered through Qt's event loop -- global
// hotkeys captured via XRecord never reach it, so Qt's timestamp is stale or
// zero and the WM treats the request as focus-stealing. Setting source = 2
// (pager / "explicit user request") together with the real hotkey timestamp
// is the EWMH-blessed way for an existing window to be brought to the front
// in response to a user action.
extern "C" void x11_activate_window(unsigned long winid, unsigned int time) {
    if (winid == 0) return;
    auto* x11 = qApp->nativeInterface<QNativeInterface::QX11Application>();
    if (!x11) return;
    Display* d = x11->display();
    if (!d) return;

    Atom active_atom = XInternAtom(d, "_NET_ACTIVE_WINDOW", False);
    Window root = DefaultRootWindow(d);

    XEvent ev;
    memset(&ev, 0, sizeof(ev));
    ev.xclient.type         = ClientMessage;
    ev.xclient.window       = static_cast<Window>(winid);
    ev.xclient.message_type = active_atom;
    ev.xclient.format       = 32;
    ev.xclient.data.l[0]    = 2;          // source: pager / explicit user request
    ev.xclient.data.l[1]    = time;       // timestamp of triggering user action
    ev.xclient.data.l[2]    = 0;          // currently active window (unknown)
    ev.xclient.data.l[3]    = 0;
    ev.xclient.data.l[4]    = 0;

    XSendEvent(d, root, False,
               SubstructureNotifyMask | SubstructureRedirectMask, &ev);
    XFlush(d);
}

#endif // WITH_X11
