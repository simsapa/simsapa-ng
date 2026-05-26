// macOS app + window activation helper for the global-hotkey lookup path.
//
// QWindow::requestActivate() only reorders windows within the current app on
// the Cocoa backend; it cannot bring a backgrounded Simsapa to the front, nor
// can it unhide (Cmd+H) or deminiaturize (yellow-button) a window. This file
// handles all three at the AppKit layer.
//
// Activation API choice: Apple's TN3147 explicitly says the new macOS 14
// `[NSApp activate]` cooperative form is for IN-APP activation requests.
// Activations that originate outside our app -- global hotkeys, URL handlers,
// Apple Events -- should continue using `activateIgnoringOtherApps:` until
// Apple provides a replacement, even on macOS 14+. The deprecation warning
// in modern SDKs is aspirational, not a behaviour change.

#ifdef __APPLE__

#import <AppKit/AppKit.h>

extern "C" void mac_activate_app_and_window(unsigned long long winid) {
    NSApplication* app = [NSApplication sharedApplication];
    if (!app) return;

    // 1. Unhide if the user pressed Cmd+H. Must happen before activate, since
    //    activate() on a hidden app is a no-op on some macOS versions.
    if ([app isHidden]) {
        [app unhide:nil];
    }

    // 2. Bring the app to the front. Suppress the deprecation diagnostic --
    //    see file header for rationale.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    [app activateIgnoringOtherApps:YES];
#pragma clang diagnostic pop

    // 3. Deminiaturize the specific lookup window if the user minimized it to
    //    the Dock, then make it the key window. Qt's winId() on cocoa returns
    //    the NSView*, from which we walk to the NSWindow.
    if (winid == 0) return;
    NSView* view = (__bridge NSView*)reinterpret_cast<void*>(winid);
    if (!view) return;
    NSWindow* win = [view window];
    if (!win) return;
    if ([win isMiniaturized]) {
        [win deminiaturize:nil];
    }
    [win makeKeyAndOrderFront:nil];
}

#endif
