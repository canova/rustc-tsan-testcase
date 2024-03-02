#![feature(thread_id_value)]
use nix::libc;
use nix::sys::signal::{self, Signal};
use rayon::ThreadPoolBuilder;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time};

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn sigprof_handler(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    RUNNING.store(false, Ordering::SeqCst);
}

fn main() {
    // Setting up the signal handler.
    let result = unsafe {
        let sig_action = signal::SigAction::new(
            signal::SigHandler::SigAction(sigprof_handler),
            signal::SaFlags::SA_RESTART,
            signal::SigSet::empty(),
        );
        signal::sigaction(signal::SIGPROF, &sig_action)
    };
    assert!(result.is_ok());

    // Now we create rayon thread pool with one thread and then get its pthread to send a signal later.
    ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .unwrap();
    let pool = ThreadPoolBuilder::default().build().unwrap();
    let get_thread_info = || {
        let pthread = nix::sys::pthread::pthread_self();
        let tid = nix::unistd::gettid();
        println!("pthread: {:?}, tid: {:?}", pthread, tid);
        (pthread, tid)
    };
    let (pthread, _) = pool.install(get_thread_info);

    // Give rayon enough time to make the threads asleep.
    thread::sleep(time::Duration::from_millis(100));

    // Send the SIGPROF signal to the sleeping thread.
    let result = nix::sys::pthread::pthread_kill(pthread, Signal::SIGPROF);
    assert!(result.is_ok());

    // Wait until we get a message back from the thread.
    while RUNNING.load(Ordering::Relaxed) {
        std::hint::spin_loop()
    }

    println!("done! the test case ended as expected!");
}
