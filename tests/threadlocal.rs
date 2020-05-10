use std::thread;
use std::sync::Arc;
use per_thread_object::ThreadLocal;


#[test]
fn test_get() {
    let tl: ThreadLocal<Box<usize>> = ThreadLocal::new();

    assert!(tl.get().is_none());

    let val = tl.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);

    let val = tl.get().unwrap();
    assert_eq!(0x42, **val);

    let val = tl.get_or(|| Box::new(0x32));
    assert_eq!(0x42, **val);
}

#[test]
fn test_thread_get() {
    let tl: ThreadLocal<Box<usize>> = ThreadLocal::new();
    let tl2 = tl.clone();
    let tl3 = tl2.clone();

    let val = tl.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);

    let j = thread::spawn(move || {
        assert!(tl2.get().is_none());

        let val = tl2.get_or(|| Box::new(0x32));
        assert_eq!(0x32, **val);

        let val = tl2.get_or(|| Box::new(0x12));
        assert_eq!(0x32, **val);
    });

    let j2 = thread::spawn(move || {
        assert!(tl3.get().is_none());

        let val = tl3.get_or(|| Box::new(0x22));
        assert_eq!(0x22, **val);

        let val = tl3.get().unwrap();
        assert_eq!(0x22, **val);

        tl3.clean();

        assert!(tl3.get().is_none());
    });

    let val = tl.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);

    j.join().unwrap();
    j2.join().unwrap();
}

#[test]
fn test_multi_obj() {
    let tla: ThreadLocal<Box<u32>> = ThreadLocal::new();
    let tlb: ThreadLocal<Box<u64>> = ThreadLocal::new();
    let tlb = Arc::new(tlb);

    assert!(tla.get().is_none());
    assert!(tlb.get().is_none());

    let tla1 = tla.clone();
    let tlb1: Arc<ThreadLocal<_>> = tlb.clone();

    let j = thread::spawn(move || {
        assert!(tla1.get().is_none());

        let val = tla1.get_or(|| Box::new(0x32));
        assert_eq!(0x32, **val);

        let val = tla1.get().unwrap();
        assert_eq!(0x32, **val);

        tla1.clean();

        let val = tla1.get_or(|| Box::new(0x12));
        assert_eq!(0x12, **val);
    });

    let j2 = thread::spawn(move || {
        assert!(tlb1.get().is_none());

        let val = tlb1.get_or(|| Box::new(0x22));
        assert_eq!(0x22, **val);

        let val = tlb1.get().unwrap();
        assert_eq!(0x22, **val);

        tlb1.clean();

        assert!(tlb1.get().is_none());
    });

    let val = tla.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);
    let val = tlb.get_or(|| Box::new(0x52));
    assert_eq!(0x52, **val);

    j.join().unwrap();
    j2.join().unwrap();

    let val = tla.get().unwrap();
    assert_eq!(0x42, **val);
    let val = tlb.get().unwrap();
    assert_eq!(0x52, **val);
}

#[test]
fn test_panic() {
    use std::panic::{ self, AssertUnwindSafe };

    struct Bar;

    impl Drop for Bar {
        fn drop(&mut self) {
            panic!()
        }
    }

    let tl: ThreadLocal<Bar> = ThreadLocal::new();

    tl.get_or(|| Bar);

    let ret = panic::catch_unwind(AssertUnwindSafe(|| {
        tl.clean();
    }));
    assert!(ret.is_err());
}
