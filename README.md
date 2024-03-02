# Rustc TSan problem with signals test case

This is a test case to demonstrate that TSan deadlocks when there is an async signal sent to a thread and that thread doesn't have any blocking functions executing at that time. This is something we encountered while running the Firefox Profiler with TSan enabled. It hangs the whole browser because of the deadlock.

Rayon uses FUTEX_WAIT syscall to wait for any new task while the thread is idle.
