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
    let tl: Arc<ThreadLocal<Box<usize>>> = Arc::new(ThreadLocal::new());
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
        let tl3 = tl3;

        assert!(tl3.get().is_none());

        let val = tl3.get_or(|| Box::new(0x22));
        assert_eq!(0x22, **val);

        let val = tl3.get().unwrap();
        assert_eq!(0x22, **val);
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
    let tla = Arc::new(tla);
    let tlb = Arc::new(tlb);

    assert!(tla.get().is_none());
    assert!(tlb.get().is_none());

    let tla1 = tla.clone();
    let tlb1: Arc<ThreadLocal<_>> = tlb.clone();

    let j = thread::spawn(move || {
        let tla1 = tla1;

        assert!(tla1.get().is_none());

        let val = tla1.get_or(|| Box::new(0x32));
        assert_eq!(0x32, **val);

        let val = tla1.get().unwrap();
        assert_eq!(0x32, **val);
    });

    let j2 = thread::spawn(move || {
        assert!(tlb1.get().is_none());

        let val = tlb1.get_or(|| Box::new(0x22));
        assert_eq!(0x22, **val);

        let val = tlb1.get().unwrap();
        assert_eq!(0x22, **val);
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
