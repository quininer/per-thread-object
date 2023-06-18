use std::ptr::NonNull;
use std::cmp::Reverse;
use std::collections::{ HashMap, BinaryHeap };
use crate::page::ThreadsRef;
use crate::loom::sync::{ Arc, Mutex };
use crate::loom::cell::UnsafeCell;

#[cfg(feature = "loom")]
use loom::{ thread_local, lazy_static };

#[cfg(feature = "shuttle")]
use shuttle::{ thread_local, lazy_static };

pub struct ThreadHandle(Arc<Mutex<HashMap<ThreadsRef, Dtor>>>);

#[cfg(not(feature = "loom"))]
#[cfg(not(feature = "shuttle"))]
static THREAD_ID_POOL: Mutex<ThreadIdPool> = Mutex::new(ThreadIdPool::new());

#[cfg(any(feature = "loom", feature = "shuttle"))]
lazy_static! {
    static ref THREAD_ID_POOL: Mutex<ThreadIdPool> = Mutex::new(ThreadIdPool::new());
}

thread_local!{
    static THREAD_STATE: ThreadState = ThreadState::new();
}

struct ThreadIdPool {
    max: usize,
    pool: Option<BinaryHeap<Reverse<usize>>>,
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
            pool: None
        }
    }

    fn alloc(&mut self) -> usize {
        if let Some(Reverse(id)) = self.pool.get_or_insert_with(BinaryHeap::new).pop() {
            id
        } else {
            let id = self.max;
            self.max = id.checked_add(1).expect("thread id overflow");
            id
        }
    }

    fn dealloc(&mut self, id: usize) {
        self.pool.get_or_insert_with(BinaryHeap::new).push(Reverse(id));
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
    fn new<T: 'static>(ptr: NonNull<UnsafeCell<Option<T>>>) -> Dtor {
        unsafe fn try_drop<T: 'static>(ptr: *mut ()) {
            let obj = &mut *ptr.cast::<UnsafeCell<Option<T>>>();
            obj.with_mut(|val| {
                let _ = { &mut *val }.take();
            });
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
        let mut list = self.list.lock().unwrap();

        for (tr, dtor) in list.drain() {
            unsafe {
                dtor.drop();

                // # Safety
                //
                // because storage will ensure that all tracked `ThreadsRef` are valid.
                tr.remove(self.id);
            }
        }

        drop(list);

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

pub unsafe fn push<T: 'static>(tr: ThreadsRef, ptr: NonNull<UnsafeCell<Option<T>>>) -> ThreadHandle {
    let dtor = Dtor::new(ptr);

    THREAD_STATE.with(|state| {
        state.list.lock()
            .unwrap()
            .insert(tr, dtor);
        ThreadHandle(Arc::clone(&state.list))
    })
}
