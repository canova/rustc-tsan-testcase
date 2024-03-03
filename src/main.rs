use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::{mem, thread, time};

static SHARED_VALUE: Lazy<Arc<AtomicI32>> = Lazy::new(|| Arc::new(AtomicI32::new(0)));

unsafe extern "C" fn sigprof_handler(
    _: libc::c_int,
    _: *mut libc::siginfo_t,
    _: *mut libc::c_void,
) {
    // Modify the shared data
    SHARED_VALUE.store(1, Ordering::SeqCst);
}

fn main() {
    let main_tid = unsafe { libc::gettid() };
    println!("[tid={}] starting", main_tid);

    // Setting up the signal handler.
    setup_signal_handler(main_tid);

    // Condvar value will be used to block the thread until the signal is received.
    let condvar = Arc::new((Mutex::new(false), Condvar::new()));
    let condvar_clone = Arc::clone(&condvar);
    // We need to know about the pthread that will be waiting on the condvar.
    let pthread = Arc::new(AtomicU64::new(0));
    let pthread_clone = Arc::clone(&pthread);

    // Spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        let tid = unsafe { libc::gettid() };
        let pthread_self = unsafe { libc::pthread_self() };
        pthread_clone.store(pthread_self, Ordering::SeqCst);

        println!("[tid={}] waiting for condvar now", tid);

        let (mutex, cvar) = &*condvar_clone;
        let mut done = mutex.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }

        println!("[tid={}] awakened!", tid);
    });

    // First we need to know the pthread that will be waiting on the condvar.
    while pthread.load(Ordering::SeqCst) == 0 {}

    let pthread = pthread.load(Ordering::SeqCst);
    println!("[tid={}] got the pthread {}", main_tid, pthread);

    // Wait a bit just to make sure it's waiting for futex now.
    thread::sleep(time::Duration::from_millis(100));

    // Send the signal to the waiting thread.
    let res = unsafe { libc::pthread_kill(pthread, libc::SIGPROF as libc::c_int) };
    println!("[tid={}] sent the signal", main_tid);
    assert!(res == 0);

    println!(
        "[tid={}] waiting for the shared value to change from the signal handler...",
        main_tid
    );
    while SHARED_VALUE.load(Ordering::SeqCst) == 0 {}

    println!("[tid={}] shared value has changed!", main_tid);
    // We notify the condvar that the value has changed.
    {
        let (lock, cvar) = &*condvar;
        let mut done = lock.lock().unwrap();
        *done = true;
        cvar.notify_one();
    }

    println!("[tid={}] notified the condvar", main_tid);

    // Wait for the thread to finish.
    handle.join().unwrap();

    println!("[tid={}] test case finished as expected!", main_tid);
}

fn setup_signal_handler(main_tid: i32) {
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
