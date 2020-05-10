use std::mem;
use std::ops::Deref;
use std::ptr::NonNull;
use std::borrow::Borrow;
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
    pub fn as_ptr(&self) -> NonNull<()> {
        self.ptr.cast()
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

impl<T> Deref for HeapRc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let inner = unsafe {  &*self.ptr.as_ptr() };
        &inner.inner
    }
}

impl<T> Clone for HeapRc<T> {
    fn clone(&self) -> HeapRc<T> {
        let inner = unsafe {  &*self.ptr.as_ptr() };
        inner.count.fetch_add(1, Ordering::Relaxed);
        HeapRc { ptr: self.ptr }
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

impl PartialEq for DropRc {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr.eq(&other.ptr)
    }
}

impl PartialOrd for DropRc {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.ptr.partial_cmp(&other.ptr)
    }
}

impl Eq for DropRc {}

impl Ord for DropRc {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

impl Borrow<NonNull<()>> for DropRc {
    #[inline]
    fn borrow(&self) -> &NonNull<()> {
        &self.ptr
    }
}

impl Drop for DropRc {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr.as_ptr());
        }
    }
}
