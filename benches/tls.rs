use std::time::Instant;
use rayon::prelude::*;
use criterion::{ criterion_main, criterion_group, Criterion, black_box };


const N: usize = 500;

fn bench_thread_local(c: &mut Criterion) {
    c.bench_function("std::thread_local!", |b| {
        thread_local!{
            static TL: u64 = 0x42;
        }

        b.iter_custom(|iters| {
            (0..iters)
                .into_par_iter()
                .map(|_| {
                    let start = Instant::now();
                    for _ in 0..N {
                        TL.with(|val| drop(black_box(val)));
                    }
                    start.elapsed()
                })
                .sum()
        });
    });

    c.bench_function("per-thread-object", |b| {
        use per_thread_object::ThreadLocal;

        let tl: ThreadLocal<u64> = ThreadLocal::new();

        b.iter_custom(|iters| {
            (0..iters)
                .into_par_iter()
                .map(|_| {
                    per_thread_object::stack_token!(token);

                    let start = Instant::now();
                    for _ in 0..N {
                        black_box(*tl.get_or_init(token, || 0x42));
                    }
                    start.elapsed()
                })
                .sum()
        });
    });

    c.bench_function("thread_local", |b| {
        use thread_local::ThreadLocal;

        let tl: ThreadLocal<u64> = ThreadLocal::new();

        b.iter_custom(|iters| {
            (0..iters)
                .into_par_iter()
                .map(|_| {
                    let start = Instant::now();
                    for _ in 0..N {
                        let val = tl.get_or(|| 0x42);
                        black_box(val);
                    }
                    start.elapsed()
                })
                .sum()
        });
    });

    c.bench_function("os-thread-local", |b| {
        use os_thread_local::ThreadLocal;

        let tl: ThreadLocal<u64> = ThreadLocal::new(|| 0x42);

        b.iter_custom(|iters| {
            (0..iters)
                .into_par_iter()
                .map(|_| {
                    let start = Instant::now();
                    for _ in 0..N {
                        tl.with(|val| drop(black_box(val)));
                    }
                    start.elapsed()
                })
                .sum()
        });
    });
}

criterion_group!(tls, bench_thread_local);
criterion_main!(tls);
