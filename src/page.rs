use std::mem;
use std::ptr::NonNull;
use std::mem::ManuallyDrop;
use std::collections::BTreeMap;
use crate::thread::ThreadHandle;
use crate::loom::cell::UnsafeCell;
use crate::loom::sync::Mutex;


pub const DEFAULT_PAGE_CAP: usize = 16;

pub struct Storage<T> {
    inner: Box<Inner<T>>
}

#[derive(Hash, Eq, PartialEq)]
pub struct ThreadsRef {
    ptr: NonNull<Mutex<BTreeMap<usize, ThreadHandle>>>
}

struct Inner<T> {
    threads: Mutex<BTreeMap<usize, ThreadHandle>>,
    fallback: Mutex<Vec<Page<T>>>,
    fastpage: [ManuallyDrop<UnsafeCell<Option<T>>>; DEFAULT_PAGE_CAP]
}

struct Page<T> {
    ptr: Box<[ManuallyDrop<UnsafeCell<Option<T>>>]>
}

impl<T> Storage<T> {
    #[inline]
    pub fn new() -> Storage<T> {
        Storage::with_threads(DEFAULT_PAGE_CAP)
    }

    pub fn with_threads(_num: usize) -> Storage<T> {
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
            threads: Mutex::new(BTreeMap::new()),
            fallback: Mutex::new(Vec::new()),
            fastpage: arr![ManuallyDrop::new(UnsafeCell::new(None)); x16]
        });

        Storage { inner }
    }

    #[inline]
    pub fn as_threads_ref(&self) -> ThreadsRef {
        ThreadsRef {
            ptr: NonNull::from(&self.inner.threads)
        }
    }

    pub fn insert_thread_handle(&self, id: usize, handle: ThreadHandle) {
        self.inner.threads.lock()
            .unwrap()
            .insert(id, handle);
    }

    #[inline]
    pub unsafe fn get(&self, id: usize) -> Option<&T> {
        let inner = &self.inner;
        let (page_id, index) = map_index(DEFAULT_PAGE_CAP, id);

        if page_id == 0 {
            inner.fastpage.get_unchecked(index)
                .with(|obj| (*obj).as_ref())
        } else {
            Storage::or_get(inner, page_id, index)
        }
    }

    #[inline]
    pub unsafe fn get_or_new(&self, id: usize) -> &mut Option<T> {
        let inner = &self.inner;
        let (page_id, index) = map_index(DEFAULT_PAGE_CAP, id);

        if page_id == 0 {
            inner.fastpage.get_unchecked(index)
                .with_mut(|obj| &mut *obj)
        } else {
            Storage::or_new(inner, page_id, index)
        }
    }

    #[cold]
    unsafe fn or_get(inner: &Inner<T>, page_id: usize, index: usize) -> Option<&T> {
        let pages = inner.fallback.lock().unwrap();
        pages.get(page_id - 1)?
            .ptr
            .get_unchecked(index)
            .with(|obj| (*obj).as_ref())
    }

    #[cold]
    unsafe fn or_new(inner: &Inner<T>, page_id: usize, index: usize) -> &mut Option<T> {
        let mut pages = inner.fallback.lock().unwrap();
        let page_id = page_id - 1;

        if page_id > pages.len() {
            pages.resize_with(page_id + 1, Page::new);
        }

        pages.get_unchecked(page_id)
            .ptr
            .get_unchecked(index)
            .with_mut(|obj| &mut *obj)
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
}

impl<T> Drop for Storage<T> {
    fn drop(&mut self) {
        let tr = self.as_threads_ref();

        let threads = {
            let mut threads = self.inner.threads.lock().unwrap();
            mem::take(&mut *threads)
        };
        for thread in threads.values() {
            unsafe {
                thread.release(&tr);
            }
        }
    }
}

impl ThreadsRef {
    pub unsafe fn remove(&self, id: usize) {
        let mut threads = (*self.ptr.as_ptr()).lock().unwrap();
        threads.remove(&id);
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
