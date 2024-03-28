#![feature(lazy_cell)]
#![feature(rustc_private)]
extern crate libc;

use std::{
    mem,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier, LazyLock,
    },
    thread, time,
};

// Create a barrier that unlocks when it reaches to 2 wait() calls.
static BARRIER: LazyLock<Arc<Barrier>> = LazyLock::new(|| Arc::new(Barrier::new(2)));

extern "C" fn sigprof_handler(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    // This second wait() call will unlock the barrier and the thread will be able to exit..
    BARRIER.wait();
}

fn main() {
    let main_tid = unsafe { libc::gettid() };
    println!("[tid={}] starting", main_tid);

    // Setting up the signal handler.
    setup_signal_handler(main_tid);

    // We need to know about the pthread that will be waiting on the condvar.
    let pthread = Arc::new(AtomicU64::new(0));
    let pthread_clone = Arc::clone(&pthread);
    // Clone the barrier too.
    let barrier_clone = Arc::clone(&BARRIER);

    // Spawn a thread to wait on the futex.
    let handle = std::thread::spawn(move || {
        // Notify the main thread about the pthread ID of this thread.
        let tid = unsafe { libc::gettid() };
        let pthread_self = unsafe { libc::pthread_self() };
        pthread_clone.store(pthread_self, Ordering::SeqCst);

        println!("[tid={}] waiting for the barrier to unlock now", tid);
        barrier_clone.wait();
        println!("[tid={}] barrier is unlocked, exiting thread", tid);
    });

    // First we need to know the pthread that will be waiting on the condvar.
    while pthread.load(Ordering::SeqCst) == 0 {}

    let pthread = pthread.load(Ordering::SeqCst);
    println!("[tid={}] got the pthread {}", main_tid, pthread);

    // Wait a bit just to make sure it's waiting for the futex now.
    thread::sleep(time::Duration::from_millis(100));

    // Send the signal to the waiting thread.
    let res = unsafe { libc::pthread_kill(pthread, libc::SIGPROF as libc::c_int) };
    println!("[tid={}] sent the signal", main_tid);
    assert!(res == 0);

    println!("[tid={}] waiting for the thread to complete...", main_tid);
    // Wait for the thread to finish.
    handle.join().unwrap();

    println!("[tid={}] test case finished as expected!", main_tid);
}

fn setup_signal_handler(main_tid: libc::pid_t) {
    // Setting up the signal handler.
    let mut s = mem::MaybeUninit::<libc::sigaction>::uninit();
    let sig_action = unsafe {
        let p = s.as_mut_ptr();
        (*p).sa_sigaction = sigprof_handler as usize;
        (*p).sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        (*p).sa_mask = {
            let mut sigset = mem::MaybeUninit::uninit();
            let _ = libc::sigemptyset(sigset.as_mut_ptr());

            sigset.assume_init()
        };

        s.assume_init()
    };
    let mut oldact = mem::MaybeUninit::<libc::sigaction>::uninit();
    let res = unsafe {
        libc::sigaction(
            libc::SIGPROF as libc::c_int,
            &sig_action as *const libc::sigaction,
            oldact.as_mut_ptr(),
        )
    };
    assert!(res == 0);
    println!("[tid={}] signal is set up", main_tid);
}
