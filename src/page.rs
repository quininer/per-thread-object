use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::cell::UnsafeCell;
use parking_lot::Mutex;
use crate::thread::ThreadHandle;


pub(crate) const DEFAULT_PAGE_CAP: usize = 16;

pub struct Pages<T> {
    ptr: NonNull<Inner<T>>
}

struct Inner<T> {
    threads: Mutex<Vec<ThreadHandle>>,
    fallback: Mutex<Vec<Page<T>>>,
    fastpage: [ManuallyDrop<UnsafeCell<Option<T>>>; DEFAULT_PAGE_CAP]
}

struct Page<T> {
    ptr: Box<[ManuallyDrop<UnsafeCell<Option<T>>>]>
}

impl<T> Pages<T> {
    #[inline]
    pub fn new() -> Pages<T> {
        Pages::with_threads(DEFAULT_PAGE_CAP)
    }

    pub fn with_threads(_num: usize) -> Pages<T> {
        macro_rules! arr {
            ( $e:expr ; x16 ) => {
                [
                    $e, $e, $e, $e,
                    $e, $e, $e, $e,
                    $e, $e, $e, $e,
                    $e, $e, $e, $e
                ]
            }
        }

        let inner = Box::new(Inner {
            threads: Mutex::new(Vec::new()),
            fallback: Mutex::new(Vec::new()),
            fastpage: arr![ManuallyDrop::new(UnsafeCell::new(None)); x16]
        });

        Pages { ptr: Box::leak(inner).into() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.ptr.as_ptr() as *const ()
    }

    pub fn push_thread_handle(&self, handle: ThreadHandle) {
        let inner = unsafe { &*self.ptr.as_ptr() };

        inner.threads.lock().push(handle);
    }

    #[inline]
    pub unsafe fn get(&self, id: usize) -> Option<&T> {
        let inner = &*self.ptr.as_ptr();
        let (page_id, index) = map_index(DEFAULT_PAGE_CAP, id);

        if page_id == 0 {
            let obj = inner.fastpage.get_unchecked(index).get();
            (*obj).as_ref()
        } else {
            Pages::or_get(inner, page_id, index)
        }
    }

    #[inline]
    pub unsafe fn get_or_new(&self, id: usize) -> &mut Option<T> {
        let inner = &*self.ptr.as_ptr();
        let (page_id, index) = map_index(DEFAULT_PAGE_CAP, id);

        let obj = if page_id == 0 {
            inner.fastpage.get_unchecked(index).get()
        } else {
            Pages::or_new(inner, page_id, index)
        };

        &mut *obj
    }

    #[cold]
    unsafe fn or_get(inner: &Inner<T>, page_id: usize, index: usize) -> Option<&T> {
        let pages = inner.fallback.lock();
        let obj = pages.get(page_id - 1)?.get(index);
        (*obj).as_ref()
    }

    #[cold]
    unsafe fn or_new(inner: &Inner<T>, page_id: usize, index: usize) -> *mut Option<T> {
        let mut pages = inner.fallback.lock();
        let page_id = page_id - 1;

        if page_id > pages.len() {
            pages.resize_with(page_id + 1, Page::new);
        }

        pages.get_unchecked(page_id).get(index)
    }
}

impl<T> Page<T> {
    fn new() -> Page<T> {
        macro_rules! arr {
            ( $e:expr ; x16 ) => {
                vec![
                    $e, $e, $e, $e,
                    $e, $e, $e, $e,
                    $e, $e, $e, $e,
                    $e, $e, $e, $e
                ]
            }
        }

        Page { ptr: arr![ManuallyDrop::new(UnsafeCell::new(None)); x16].into_boxed_slice() }
    }

    #[inline]
    unsafe fn get(&self, index: usize) -> *mut Option<T> {
        self.ptr.get_unchecked(index).get()
    }
}

impl<T> Drop for Pages<T> {
    fn drop(&mut self) {
        {
            let addr = self.as_ptr() as usize;
            let inner = unsafe {  &*self.ptr.as_ptr() };

            let threads = inner.threads.lock();
            for thread in &*threads {
                unsafe {
                    thread.release(addr);
                }
            }
        }

        unsafe {
            Box::from_raw(self.ptr.as_ptr());
        }
    }
}

#[inline]
fn map_index(cap: usize, n: usize) -> (usize, usize) {
    if n <= cap {
        (0, n)
    } else {
        let i = n / cap;
        (i, n - (i * cap))
    }
}
