use std::mem;
use std::ptr::NonNull;
use std::collections::HashMap;
use crate::page::{ DEFAULT_PAGE_CAP, ThreadsRef };
use crate::loom::sync::{ Arc, Mutex };

#[cfg(feature = "loom")]
use loom::thread_local;


pub struct ThreadHandle(Arc<Mutex<HashMap<ThreadsRef, Dtor>>>);

#[cfg(not(feature = "loom"))]
static THREAD_ID_POOL: Mutex<ThreadIdPool> = Mutex::new(ThreadIdPool::new());

#[cfg(feature = "loom")]
loom::lazy_static!{
    static ref THREAD_ID_POOL: Mutex<ThreadIdPool> = Mutex::new(ThreadIdPool::new());
}

thread_local!{
    static THREAD_STATE: ThreadState = ThreadState::new();
}

struct ThreadIdPool {
    max: usize,
    small_freelist: Vec<usize>, // TODO use heapless vec
    slow_freelist: Vec<usize>
}

struct ThreadState {
    id: usize,
    list: Arc<Mutex<HashMap<ThreadsRef, Dtor>>>
}

struct Dtor {
    ptr: NonNull<()>,
    drop: unsafe fn(*mut ())
}

impl ThreadIdPool {
    const fn new() -> ThreadIdPool {
        ThreadIdPool {
            max: 0,
            small_freelist: Vec::new(),
            slow_freelist: Vec::new()
        }
    }

    fn alloc(&mut self) -> usize {
        if let Some(id) = self.small_freelist.pop()
            .or_else(|| self.slow_freelist.pop())
        {
            if self.slow_freelist.capacity() != 0
                && self.slow_freelist.is_empty()
                && self.small_freelist.len() < DEFAULT_PAGE_CAP / 2
            {
                self.slow_freelist.shrink_to_fit();
            }

            id
        } else {
            let id = self.max;
            self.max = id.checked_add(1).expect("thread id overflow");
            id
        }
    }

    fn dealloc(&mut self, id: usize) {
        if id <= DEFAULT_PAGE_CAP {
            self.small_freelist.push(id);
        } else {
            self.slow_freelist.push(id)
        }
    }
}

impl ThreadState {
    fn new() -> ThreadState {
        ThreadState {
            id: THREAD_ID_POOL.lock().unwrap().alloc(),
            list: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

impl Dtor {
    fn new<T: 'static>(ptr: NonNull<Option<T>>) -> Dtor {
        unsafe fn try_drop<T: 'static>(ptr: *mut ()) {
            let obj = &mut *ptr.cast::<Option<T>>();
            obj.take();
        }

        Dtor {
            ptr: ptr.cast(),
            drop: try_drop::<T>
        }
    }

    unsafe fn drop(&self) {
        (self.drop)(self.ptr.as_ptr())
    }
}

impl Drop for ThreadState {
    fn drop(&mut self) {
        // take list, avoid double free
        let list = {
            let mut list = self.list.lock().unwrap();
            mem::take(&mut *list)
        };

        for (tr, dtor) in list.iter() {
            unsafe {
                dtor.drop();

                // # Safety
                //
                // because storage will ensure that all tracked `ThreadsRef` are valid.
                tr.remove(self.id);
            }
        }

        THREAD_ID_POOL.lock().unwrap()
            .dealloc(self.id);
    }
}

impl ThreadHandle {
    pub unsafe fn release(&self, tr: &ThreadsRef) {
        let dtor = {
            self.0.lock()
                .unwrap()
                .remove(tr)
        };

        if let Some(dtor) = dtor {
            dtor.drop();
        }
    }
}

#[inline]
pub fn get() -> usize {
    THREAD_STATE.with(|state| state.id)
}

pub unsafe fn push<T: 'static>(tr: ThreadsRef, ptr: NonNull<Option<T>>) -> ThreadHandle {
    let dtor = Dtor::new(ptr);

    THREAD_STATE.with(|state| {
        state.list.lock()
            .unwrap()
            .insert(tr, dtor);
        ThreadHandle(Arc::clone(&state.list))
    })
}
