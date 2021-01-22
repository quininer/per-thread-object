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

        assert!(tl.try_with(|_| ()).is_none());

        let val = tl.with_or(|val| **val, || Box::new(0x42));
        assert_eq!(0x42, val);

        let tl2 = tl.clone();
        thread::spawn(move || {
            let val = tl2.with_or(|val| **val, || Box::new(0x22));
            assert_eq!(0x22, val);
        });

        let val = tl.with(|val| **val);
        assert_eq!(0x42, val);

        let val = tl.with_or(|val| **val, || Box::new(0x32));
        assert_eq!(0x42, val);
    });
}

#[test]
fn test_thread_get() {
    loom::model(|| {
        let tl: Arc<ThreadLocal<Box<usize>>> = Arc::new(ThreadLocal::new());
        let tl2 = tl.clone();
        let tl3 = tl2.clone();

        let val = tl.with_or(|val| **val, || Box::new(0x42));
        assert_eq!(0x42, val);

        let j = thread::spawn(move || {
            assert!(tl2.try_with(|_| ()).is_none());

            let val = tl2.with_or(|val| **val, || Box::new(0x32));
            assert_eq!(0x32, val);

            let val = tl2.with_or(|val| **val, || Box::new(0x12));
            assert_eq!(0x32, val);
        });

        let j2 = thread::spawn(move || {
            let tl3 = tl3;

            assert!(tl3.try_with(|_| ()).is_none());

            let val = tl3.with_or(|val| **val, || Box::new(0x22));
            assert_eq!(0x22, val);

            let val = tl3.with(|val| **val);
            assert_eq!(0x22, val);
        });

        let val = tl.with_or(|val| **val, || Box::new(0x42));
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

        assert!(tla.try_with(|_| ()).is_none());
        assert!(tlb.try_with(|_| ()).is_none());

        let tla1 = tla.clone();
        let tlb1: Arc<ThreadLocal<_>> = tlb.clone();

        let j = thread::spawn(move || {
            let tla1 = tla1;

            assert!(tla1.try_with(|_| ()).is_none());

            let val = tla1.with_or(|val| **val, || Box::new(0x32));
            assert_eq!(0x32, val);

            let val = tla1.with(|val| **val);
            assert_eq!(0x32, val);
        });

        let j2 = thread::spawn(move || {
            assert!(tlb1.try_with(|_| ()).is_none());

            let val = tlb1.with_or(|val| **val, || Box::new(0x22));
            assert_eq!(0x22, val);

            let val = tlb1.with(|val| **val);
            assert_eq!(0x22, val);
        });

        let val = tla.with_or(|val| **val, || Box::new(0x42));
        assert_eq!(0x42, val);
        let val = tlb.with_or(|val| **val, || Box::new(0x52));
        assert_eq!(0x52, val);

        j.join().unwrap();
        j2.join().unwrap();

        let val = tla.with(|val| **val);
        assert_eq!(0x42, val);
        let val = tlb.with(|val| **val);
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
                    let val = tla.with_or(|val| **val, || Box::new(i as u32));
                    assert_eq!(i as u32, val);

                    let val = tlb.with_or(|val| **val, || Box::new(i as u64));
                    assert_eq!(i as u64, val);
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.join().unwrap();
        }
    });
}
