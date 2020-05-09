use std::mem;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{ AtomicUsize, Ordering };


pub struct HeapRc<T> {
    ptr: NonNull<InnerRc<T>>
}

pub struct DropRc {
    ptr: NonNull<()>,
    drop: unsafe fn(*mut ())
}

struct InnerRc<T> {
    count: AtomicUsize,
    inner: T
}

impl<T> HeapRc<T> {
    pub fn new(t: T) -> HeapRc<T> {
        let inner = Box::new(InnerRc {
            count: AtomicUsize::new(1),
            inner: t
        });

        HeapRc { ptr: Box::leak(inner).into() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.ptr.as_ptr() as *const _
    }

    pub fn into_droprc(self) -> DropRc {
        unsafe fn drop_inner_rc<T>(ptr: *mut ()) {
            let ptr = ptr as *mut InnerRc<T>;
            let inner = &*ptr;

            if inner.count.fetch_sub(1, Ordering::Relaxed) == 1 {
                Box::from_raw(ptr);
            }
        }

        let ptr = self.ptr.cast();
        mem::forget(self);

        DropRc {
            ptr,
            drop: drop_inner_rc::<T>
        }
    }
}

impl DropRc {
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }
}

impl<T> Deref for HeapRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let inner = unsafe {  &*self.ptr.as_ptr() };
        &inner.inner
    }
}

impl<T> Clone for HeapRc<T> {
    fn clone(&self) -> HeapRc<T> {
        let inner = unsafe {  &*self.ptr.as_ptr() };
        inner.count.fetch_add(1, Ordering::Relaxed);
        HeapRc { ptr: self.ptr.clone() }
    }
}

impl<T> Drop for HeapRc<T> {
    fn drop(&mut self) {
        let inner = unsafe {  &*self.ptr.as_ptr() };

        if inner.count.fetch_sub(1, Ordering::Relaxed) == 1 {
            unsafe {
                Box::from_raw(self.ptr.as_ptr());
            }
        }
    }
}

impl Drop for DropRc {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr.as_ptr());
        }
    }
}
