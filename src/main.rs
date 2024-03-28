use nix::{
    libc,
    sys::signal::{self, Signal},
    unistd::Pid,
};
use once_cell::sync::Lazy;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    },
    thread, time,
};

// Create a barrier that unlocks when it reaches to 2 wait() calls.
static BARRIER: Lazy<Arc<Barrier>> = Lazy::new(|| Arc::new(Barrier::new(2)));

extern "C" fn sigprof_handler(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    // Notify the barrier which will unlock the waiting thread as it reaches number 2.
    BARRIER.wait();
}

fn main() {
    let main_tid = nix::unistd::gettid();
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
        let tid = nix::unistd::gettid();
        let pthread_self = nix::sys::pthread::pthread_self();
        pthread_clone.store(pthread_self, Ordering::SeqCst);

        println!("[tid={}] waiting for the barrier to unlock now", tid);
        barrier_clone.wait();
        println!("[tid={}] barrier is unlocked, exiting thread", tid);
    });

    // First we need to know the pthread that will be waiting on the condvar.
    while pthread.load(Ordering::SeqCst) == 0 {}

    let pthread = pthread.load(Ordering::SeqCst);
    println!("[tid={}] got the pthread {}", main_tid, pthread);

    // Wait a bit just to make sure it's waiting for futex now.
    thread::sleep(time::Duration::from_millis(100));

    // Send the signal to the waiting thread.
    let result = nix::sys::pthread::pthread_kill(pthread, Signal::SIGPROF);
    println!("[tid={}] sent the signal", main_tid);
    assert!(result.is_ok());

    println!("[tid={}] waiting for the thread to complete...", main_tid);
    // Wait for the thread to finish.
    handle.join().unwrap();

    println!("[tid={}] test case finished as expected!", main_tid);
}

fn setup_signal_handler(main_tid: Pid) {
    // Setting up the signal handler.
    let sig_action = signal::SigAction::new(
        signal::SigHandler::SigAction(sigprof_handler),
        signal::SaFlags::SA_RESTART,
        signal::SigSet::empty(),
    );
    let result = unsafe { signal::sigaction(signal::SIGPROF, &sig_action) };
    assert!(result.is_ok());
    println!("[tid={}] signal is set up", main_tid);
}
