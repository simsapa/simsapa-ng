#ifndef WAKE_LOCK_H
#define WAKE_LOCK_H

extern "C++" {
    bool acquire_wake_lock();
    void release_wake_lock();
}

#endif
