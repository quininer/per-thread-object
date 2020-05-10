use std::mem;
use std::ptr::NonNull;
use std::cell::UnsafeCell;
use std::sync::atomic::{ AtomicUsize, Ordering };
use parking_lot::Mutex;
use crate::rc::DropRc;


pub(crate) const PAGE_CAP: usize = 64;

pub struct Pages<T> {
    ptr: NonNull<Inner<T>>
}

struct Inner<T> {
    count: AtomicUsize,
    fallback: Mutex<Vec<Page<T>>>,
    fastpage: [UnsafeCell<Option<T>>; PAGE_CAP],
}

struct Page<T> {
    ptr: Box<[UnsafeCell<Option<T>>; PAGE_CAP]>
}

macro_rules! arr {
    ( $e:expr ; x64 ) => {
        [
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e,
        ]
    }
}

impl<T> Pages<T> {
    pub fn new() -> Pages<T> {
        let inner = Box::new(Inner {
            count: AtomicUsize::new(1),
            fallback: Mutex::new(Vec::new()),
            fastpage: arr![UnsafeCell::new(None); x64]
        });

        Pages { ptr: Box::leak(inner).into() }
    }

    pub unsafe fn get(&self, id: usize) -> Option<&T> {
        let inner = &*self.ptr.as_ptr();
        let (page_id, index) = map_index(id);

        let obj = if page_id == 0 {
            inner.fastpage.get_unchecked(index).get()
        } else {
            let pages = inner.fallback.lock();
            pages.get(page_id - 1)?.get(index)
        };

        (&*obj).as_ref()
    }

    pub unsafe fn get_or_new(&self, id: usize) -> &mut Option<T> {
        let inner = &*self.ptr.as_ptr();
        let (page_id, index) = map_index(id);

        let obj = if page_id == 0 {
            inner.fastpage.get_unchecked(index).get()
        } else {
            let mut pages = inner.fallback.lock();
            let page_id = page_id - 1;

            if page_id > pages.len() {
                pages.resize_with(page_id + 1, Page::new);
            }

            pages.get_unchecked(page_id).get(index)
        };

        &mut *obj
    }

    pub fn into_droprc(self) -> DropRc {
        unsafe fn drop_inner_rc<T>(ptr: *mut ()) {
            let ptr = ptr as *mut Inner<T>;
            let inner = &*ptr;

            if inner.count.fetch_sub(1, Ordering::Relaxed) == 1 {
                Box::from_raw(ptr);
            }
        }

        let ptr = self.ptr.cast();
        mem::forget(self);

        unsafe {
            DropRc::new(ptr, drop_inner_rc::<T>)
        }
    }
}

impl<T> Page<T> {
    #[cold]
    fn new() -> Page<T> {
        Page { ptr: Box::new(arr![UnsafeCell::new(None); x64]) }
    }

    #[inline]
    unsafe fn get(&self, index: usize) -> *mut Option<T> {
        self.ptr.get_unchecked(index).get()
    }
}

impl<T> Clone for Pages<T> {
    fn clone(&self) -> Pages<T> {
        let inner = unsafe {  &*self.ptr.as_ptr() };
        inner.count.fetch_add(1, Ordering::Relaxed);
        Pages { ptr: self.ptr }
    }
}

impl<T> Drop for Pages<T> {
    fn drop(&mut self) {
        let inner = unsafe {  &*self.ptr.as_ptr() };

        if inner.count.fetch_sub(1, Ordering::Relaxed) == 1 {
            unsafe {
                Box::from_raw(self.ptr.as_ptr());
            }
        }
    }
}

#[inline]
fn map_index(n: usize) -> (usize, usize) {
    if n <= PAGE_CAP {
        (0, n)
    } else {
        let i = n / PAGE_CAP;
        (i, n - (i * PAGE_CAP))
    }
}
