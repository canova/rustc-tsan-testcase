use libc::{c_void, syscall};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::Arc;
use std::{mem, ptr, thread, time};

static FUTEX_VALUE: Lazy<Arc<AtomicI32>> = Lazy::new(|| Arc::new(AtomicI32::new(0)));

unsafe extern "C" fn sigprof_handler(
    _: libc::c_int,
    _: *mut libc::siginfo_t,
    _: *mut libc::c_void,
) {
    // Modify the shared data
    let new_value = 1;
    FUTEX_VALUE.store(new_value, Ordering::SeqCst);

    // Wake up the waiting thread
    let futex_addr = FUTEX_VALUE.as_ptr() as *mut libc::c_void;
    unsafe {
        syscall(
            libc::SYS_futex,
            futex_addr,
            libc::FUTEX_WAKE,
            new_value,
            ptr::null::<c_void>(),
            ptr::null::<c_void>(),
            0,
        );
    }
}

fn main() {
    let main_tid = unsafe { libc::gettid() };
    println!("[tid={}] starting", main_tid);
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

    let futex_value_clone = Arc::clone(&FUTEX_VALUE);

    let pthread = Arc::new(AtomicU64::new(0));
    let pthread_clone = Arc::clone(&pthread);

    // Spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        let tid = unsafe { libc::gettid() };
        let pthread_self = unsafe { libc::pthread_self() };
        pthread_clone.store(pthread_self, Ordering::SeqCst);
        println!("[tid={}] waiting for futex", tid);
        let futex_addr = futex_value_clone.as_ptr() as *mut libc::c_void;
        unsafe {
            syscall(
                libc::SYS_futex,
                futex_addr,
                libc::FUTEX_WAIT,
                0,
                ptr::null::<c_void>(),
                ptr::null::<c_void>(),
                0,
            )
        };
        println!("[tid={}] awakened!", tid);
    });

    while pthread.load(Ordering::SeqCst) == 0 {}
    println!(
        "[tid={}] got the pthread {}",
        main_tid,
        pthread.load(Ordering::SeqCst)
    );

    // Wait a bit just to make sure it's waiting for futex now.
    thread::sleep(time::Duration::from_millis(100));

    let res =
        unsafe { libc::pthread_kill(pthread.load(Ordering::SeqCst), libc::SIGPROF as libc::c_int) };
    println!("[tid={}] sent the signal", main_tid);
    assert!(res == 0);

    // Wait for the thread to finish.
    handle.join().unwrap();

    println!("[tid={}] test case finished as expected!", main_tid);
}
