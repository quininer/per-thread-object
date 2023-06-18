use std::mem;
use std::ptr::NonNull;
use std::mem::ManuallyDrop;
use std::collections::BTreeMap;
use crossbeam_utils::CachePadded;
use crate::thread::ThreadHandle;
use crate::loom::cell::UnsafeCell;
use crate::loom::sync::Mutex;
use crate::util::BoxTail;


pub struct Storage<T> {
    inner: BoxTail<Inner<T>, FastPageElem<T>>
}

#[derive(Hash, Eq, PartialEq)]
pub struct ThreadsRef {
    ptr: NonNull<Mutex<BTreeMap<usize, ThreadHandle>>>
}

struct Inner<T> {
    threads: Mutex<BTreeMap<usize, ThreadHandle>>,
    fallback: Mutex<Vec<Page<T>>>,
}

type FastPageElem<T> = CachePadded<ManuallyDrop<UnsafeCell<Option<T>>>>;

struct Page<T> {
    ptr: Box<[ManuallyDrop<UnsafeCell<Option<T>>>]>
}

impl<T> Storage<T> {
    pub fn with_threads(num: usize) -> Storage<T> {
        let inner = BoxTail::new(
            Inner {
                threads: Mutex::new(BTreeMap::new()),
                fallback: Mutex::new(Vec::new()),
            },
            num,
            |ptr: *mut FastPageElem<T>| unsafe {
                ptr.write(CachePadded::new(ManuallyDrop::new(UnsafeCell::new(None))));
            }
        );

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
        let (page_id, index) = map_index(inner.array_len(), id);

        if page_id == 0 {
            inner.array().get_unchecked(index)
                .with(|obj| (*obj).as_ref())
        } else {
            Storage::or_get(inner, page_id, index)
        }
    }

    #[inline]
    pub unsafe fn get_or_new(&self, id: usize) -> NonNull<UnsafeCell<Option<T>>> {
        let inner = &self.inner;
        let (page_id, index) = map_index(inner.array_len(), id);

        if page_id == 0 {
            let ptr = inner.array().get_unchecked(index);
            let ptr = &***ptr as *const UnsafeCell<Option<_>>;
            NonNull::new_unchecked(ptr as *mut _)
        } else {
            Storage::or_new(inner, inner.array_len(), page_id, index)
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
    unsafe fn or_new(inner: &Inner<T>, arr_len: usize, page_id: usize, index: usize)
        -> NonNull<UnsafeCell<Option<T>>>
    {
        let mut pages = inner.fallback.lock().unwrap();
        let page_id = page_id - 1;

        if page_id >= pages.len() {
            pages.resize_with(page_id + 1, || Page::new(arr_len));
        }

        let ptr = pages.get_unchecked(page_id)
            .ptr
            .get_unchecked(index);
        let ptr = &**ptr as *const UnsafeCell<Option<_>>;
        NonNull::new_unchecked(ptr as *mut _)
    }
}

impl<T> Page<T> {
    fn new(arr_len: usize) -> Page<T> {
        let arr = (0..arr_len)
            .map(|_| ManuallyDrop::new(UnsafeCell::new(None)))
            .collect::<Vec<_>>();
        Page { ptr: arr.into_boxed_slice() }
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
    if n < cap {
        (0, n)
    } else {
        let i = n / cap;
        let rem = n % cap;
        (i, rem)
    }
}
