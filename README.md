# Rustc TSan problem with signals test case

This is a test case to demonstrate that TSan deadlocks when there is an async signal sent to a thread and that thread doesn't have any blocking functions executing at that time. This is something we encountered while running the Firefox Profiler with TSan enabled. It hangs the whole browser because of the deadlock.

Rayon uses `Condvar.wait()` which uses `FUTEX_WAIT` syscall to wait for any new task while the thread is idle. And llvm backend doesn't think that this is a blocking call. That's why it tries to delay the delivery of the signal until it find another blocking call, which never happens because it always waits.

## Build and run

First run the program without TSan to make sure that it runs as expected:

```
cargo run
```

Then run the program with TSan enabled:

```
RUSTFLAGS="-Zsanitizer=thread" cargo run -Zbuild-std --target x86_64-unknown-linux-gnu
```

Note that `-Zbuild-std` flag requires to add a `--target`. Update the target to match your system.

The expected behavior when you run this is to finish the program like when TSan is not enabled. But it hangs indefinitely instead.

This hang happens because `sigprof_handler` never gets executed as explained above.

Here's the stack of the spawned thread while waiting:

```
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x000055555571d926 in std::sys::pal::unix::futex::futex_wait (futex=0x720800000038, expected=0, timeout=...) at src/sys/pal/unix/futex.rs:62
#2  0x00005555556cf45b in std::sys::locks::condvar::futex::Condvar::wait_optional_timeout (self=0x720800000038, mutex=0x720800000030, timeout=...) at src/sys/locks/condvar/futex.rs:49
#3  0x00005555556cf3b2 in std::sys::locks::condvar::futex::Condvar::wait (self=0x720800000038, mutex=0x720800000030) at src/sys/locks/condvar/futex.rs:33
#4  0x0000555555602986 in std::sync::condvar::Condvar::wait<bool> (self=0x720800000038, guard=...)
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sync/condvar.rs:189
#5  0x0000555555602647 in rustc_tsan_testcase::main::{closure#0} () at src/main.rs:41
#6  0x0000555555600e31 in std::sys_common::backtrace::__rust_begin_short_backtrace<rustc_tsan_testcase::main::{closure_env#0}, ()> (f=...)
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys_common/backtrace.rs:155
#7  0x0000555555604551 in std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure#0}<rustc_tsan_testcase::main::{closure_env#0}, ()> ()
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/thread/mod.rs:528
#8  0x00005555556011a1 in core::panic::unwind_safe::{impl#23}::call_once<(), std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<rustc_tsan_testcase::main::{closure_env#0}, ()>> (self=...)
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/panic/unwind_safe.rs:272
#9  0x00005555555fcc81 in std::panicking::try::do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<rustc_tsan_testcase::main::{closure_env#0}, ()>>, ()> (data=0x7ffff47fd308) at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/panicking.rs:552
#10 0x00005555555fcee9 in __rust_try ()
#11 0x00005555555fcb9c in std::panicking::try<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<rustc_tsan_testcase::main::{closure_env#0}, ()>>> (f=...) at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/panicking.rs:516
#12 0x00005555555fd3f1 in std::panic::catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<rustc_tsan_testcase::main::{closure_env#0}, ()>>, ()> (f=...) at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/panic.rs:146
#13 0x00005555556042ad in std::thread::{impl#0}::spawn_unchecked_::{closure#1}<rustc_tsan_testcase::main::{closure_env#0}, ()> ()
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/thread/mod.rs:527
#14 0x00005555555fe172 in core::ops::function::FnOnce::call_once<std::thread::{impl#0}::spawn_unchecked_::{closure_env#1}<rustc_tsan_testcase::main::{closure_env#0}, ()>, ()> ()
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ops/function.rs:250
#15 0x00005555556c1486 in alloc::boxed::{impl#47}::call_once<(), dyn core::ops::function::FnOnce<(), Output=()>, alloc::alloc::Global> (self=..., args=())
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/boxed.rs:2016
#16 0x00005555556c138c in alloc::boxed::{impl#47}::call_once<(), alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output=()>, alloc::alloc::Global>, alloc::alloc::Global> (self=0x720400000010, args=())
    at /home/canova/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/boxed.rs:2016
#17 0x000055555573dc69 in std::sys::pal::unix::thread::{impl#2}::new::thread_start (main=0x720400000010) at src/sys/pal/unix/thread.rs:108
#18 0x000055555557418f in __tsan_thread_start_func () at /rustc/llvm/src/llvm-project/compiler-rt/lib/tsan/rtl/tsan_interceptors_posix.cpp:1012
#19 0x00007ffff7c94ac3 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:442
#20 0x00007ffff7d26850 in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:81
```

## Why not just Compiler Explorer or Rust Playground test case instead?

Unfortunately we can't use Compiler Explorer because it only supports libraries for release versions of Rust and TSan requires nightly. Also Rust playground doesn't support TSan or adding any flags to the build process. But if you wan't to see Compiler Explorer without TSan, [here's the link](https://godbolt.org/z/TM6E1bMhW) (with libc instead of nix library since they don't have nix library there. Filed a bug for that [here](https://github.com/compiler-explorer/compiler-explorer/issues/6212)).
