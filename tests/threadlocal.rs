#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::thread;
    pub use std::sync;

    pub fn model<F>(f: F)
    where
        F: Fn() + Sync + Send + 'static
    {
        f()
    }
}

use loom::thread;
use loom::sync::Arc;
use per_thread_object::ThreadLocal;


#[test]
fn test_get() {
    loom::model(|| {
        let tl: ThreadLocal<Box<usize>> = ThreadLocal::new();
        let tl = Arc::new(tl);

        per_thread_object::stack_token!(token);

        assert!(tl.get(token).is_none());

        let val = **tl.get_or_init(token, || Box::new(0x42));
        assert_eq!(0x42, val);

        let tl2 = tl.clone();
        thread::spawn(move || {
            per_thread_object::stack_token!(token);

            let val = **tl2.get_or_init(token, || Box::new(0x22));
            assert_eq!(0x22, val);
        });

        let val = **tl.get(token).unwrap();
        assert_eq!(0x42, val);

        let val = **tl.get_or_init(token, || Box::new(0x32));
        assert_eq!(0x42, val);
    });
}

#[test]
fn test_thread_get() {
    loom::model(|| {
        let tl: Arc<ThreadLocal<Box<usize>>> = Arc::new(ThreadLocal::new());
        let tl2 = tl.clone();
        let tl3 = tl2.clone();

        per_thread_object::stack_token!(token);

        let val = **tl.get_or_init(token, || Box::new(0x42));
        assert_eq!(0x42, val);

        let j = thread::spawn(move || {
            per_thread_object::stack_token!(token);

            assert!(tl2.get(token).is_none());

            let val = **tl2.get_or_init(token, || Box::new(0x32));
            assert_eq!(0x32, val);

            let val = **tl2.get_or_init(token, || Box::new(0x12));
            assert_eq!(0x32, val);
        });

        let j2 = thread::spawn(move || {
            let tl3 = tl3;

            per_thread_object::stack_token!(token);

            assert!(tl3.get(token).is_none());

            let val = **tl3.get_or_init(token, || Box::new(0x22));
            assert_eq!(0x22, val);

            let val = **tl3.get(token).unwrap();
            assert_eq!(0x22, val);
        });

        let val = **tl.get_or_init(token, || Box::new(0x42));
        assert_eq!(0x42, val);

        j.join().unwrap();
        j2.join().unwrap();
    });
}

#[test]
fn test_multi_obj() {
    loom::model(|| {
        let tla: ThreadLocal<Box<u32>> = ThreadLocal::new();
        let tlb: ThreadLocal<Box<u64>> = ThreadLocal::new();
        let tla = Arc::new(tla);
        let tlb = Arc::new(tlb);

        per_thread_object::stack_token!(token);

        assert!(tla.get(token).is_none());
        assert!(tlb.get(token).is_none());

        let tla1 = tla.clone();
        let tlb1: Arc<ThreadLocal<_>> = tlb.clone();

        let j = thread::spawn(move || {
            let tla1 = tla1;

            per_thread_object::stack_token!(token);

            assert!(tla1.get(token).is_none());

            let val = **tla1.get_or_init(token, || Box::new(0x32));
            assert_eq!(0x32, val);

            let val = **tla1.get(token).unwrap();
            assert_eq!(0x32, val);
        });

        let j2 = thread::spawn(move || {
            per_thread_object::stack_token!(token);

            assert!(tlb1.get(token).is_none());

            let val = **tlb1.get_or_init(token, || Box::new(0x22));
            assert_eq!(0x22, val);

            let val = **tlb1.get(token).unwrap();
            assert_eq!(0x22, val);
        });

        let val = **tla.get_or_init(token, || Box::new(0x42));
        assert_eq!(0x42, val);
        let val = **tlb.get_or_init(token, || Box::new(0x52));
        assert_eq!(0x52, val);

        j.join().unwrap();
        j2.join().unwrap();

        let val = **tla.get(token).unwrap();
        assert_eq!(0x42, val);
        let val = **tlb.get(token).unwrap();
        assert_eq!(0x52, val);
    });
}

#[test]
fn test_more_thread() {
    loom::model(|| {
        let tla: ThreadLocal<Box<u32>> = ThreadLocal::new();
        let tlb: ThreadLocal<Box<u64>> = ThreadLocal::new();
        let tla = Arc::new(tla);
        let tlb = Arc::new(tlb);

        #[cfg(not(feature = "loom"))]
        let n = 33;

        #[cfg(feature = "loom")]
        let n = 3;

        let handles = (0..n)
            .map(|i| {
                let tla = tla.clone();
                let tlb = tlb.clone();

                thread::spawn(move || {
                    per_thread_object::stack_token!(token);

                    let val = **tla.get_or_init(token, || Box::new(i as u32));
                    assert_eq!(i as u32, val);

                    let val = **tlb.get_or_init(token, || Box::new(i as u64));
                    assert_eq!(i as u64, val);
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.join().unwrap();
        }
    });
}

#[test]
fn test_loop_thread() {
    use per_thread_object::ThreadLocal;

    let tl: ThreadLocal<u64> = ThreadLocal::new();

    std::thread::scope(|s| {
        for _ in 0..64 { // must > DEFAULT_PAGE_CAP
            s.spawn(|| {
                per_thread_object::stack_token!(token);

                for _ in 0..100 {
                    tl.get_or_init(token, || 0x42);
                }
            });
        }
    });
}
