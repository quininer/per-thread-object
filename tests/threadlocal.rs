use per_object_thread_local::ThreadLocal;


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
    use std::thread;

    let tl: ThreadLocal<Box<usize>> = ThreadLocal::new();
    let tl2 = tl.clone();
    let tl3 = tl2.clone();

    let val = tl.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);

    let j = thread::spawn(move || {
        assert!(tl2.get().is_none());

        let val = tl2.get_or(|| Box::new(0x32));
        assert_eq!(0x32, **val);
    });

    let j2 = thread::spawn(move || {
        assert!(tl3.get().is_none());

        let val = tl3.get_or(|| Box::new(0x22));
        assert_eq!(0x22, **val);

        assert!(tl3.get().is_some());

        tl3.clean();

        assert!(tl3.get().is_none());
    });

    let val = tl.get_or(|| Box::new(0x42));
    assert_eq!(0x42, **val);

    j.join().unwrap();
    j2.join().unwrap();
}
