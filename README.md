# Per-object thread-local storage

Efficient per-object thread-local storage implementation.

Unlike `thread_local` crate, it will normally release the object at the end of thread.
even though it will release object in other threads, but it will not let other threads use old object
(see [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=cb3153e9ef7793f192e7b905b3f5e6bb)
and [document](https://github.com/Amanieu/thread_local-rs/blob/34011020194908f3aa852cac59a83e81a325767e/src/lib.rs#L28)).

Unlike `os-thread-local` crate, it only uses the `std::thread_local!` abstraction of rust standard library,
and does not use any other platform-related APIs.
This means that its capacity is not limited by `PTHREAD_KEYS_MAX`.

And its performance is relatively good,
value access for less than `N` threads is completely lock-free, and has `O(1)` time complexity.
But since we store thread id in `std::thread_local!`, so we will be slightly slower than `std::thread_local!`.

```
std::thread_local!      time:   [1.5702 us 1.5725 us 1.5751 us]
                        change: [-3.6008% -0.8194% +1.2594%] (p = 0.58 > 0.05)
                        No change in performance detected.
Found 7 outliers among 100 measurements (7.00%)
  7 (7.00%) high severe

per-thread-object       time:   [1.6411 us 1.6545 us 1.6697 us]
                        change: [-0.4540% +1.1403% +2.6958%] (p = 0.16 > 0.05)
                        No change in performance detected.
Found 9 outliers among 100 measurements (9.00%)
  6 (6.00%) high mild
  3 (3.00%) high severe

thread_local            time:   [2.7854 us 2.7996 us 2.8142 us]
                        change: [-3.9626% -3.0122% -1.9548%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe

os-thread-local         time:   [1.5628 us 1.5708 us 1.5801 us]
                        change: [-2.7584% +0.1343% +3.1291%] (p = 0.94 > 0.05)
                        No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
  2 (2.00%) high mild
  8 (8.00%) high severe
```

# License

This project is licensed under [the MIT license](LICENSE).
