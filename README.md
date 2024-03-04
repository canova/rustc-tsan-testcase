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

## Why not just Compiler Explorer or Rust Playground test case instead?

Unfortunately we can't use Compiler Explorer because it only supports libraries for release versions of Rust and TSan requires nightly. Also Rust playground doesn't support TSan or adding any flags to the build process. But if you wan't to see Compiler Explorer without TSan, [here's the link](https://godbolt.org/z/TM6E1bMhW) (with libc instead of nix library since they don't have nix library there. Filed a bug for that [here](https://github.com/compiler-explorer/compiler-explorer/issues/6212)).
