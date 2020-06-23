use std::sync::{ Arc, Weak };
use std::ptr::NonNull;
use std::collections::BTreeMap;
use parking_lot::{ lock_api::RawMutex as _, Mutex, RawMutex };
use crate::page::DEFAULT_PAGE_CAP;


pub struct ThreadHandle(Weak<Mutex<DtorList>>);

type DtorList = BTreeMap<usize, Dtor>;

static THREAD_ID_POOL: Mutex<ThreadIdPool> =
    Mutex::const_new(RawMutex::INIT, ThreadIdPool::new());

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
    list: Arc<Mutex<DtorList>>
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
            id: THREAD_ID_POOL.lock().alloc(),
            list: Arc::new(Mutex::new(BTreeMap::new()))
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
        let list = self.list.lock();

        for dtor in list.values() {
            unsafe {
                dtor.drop();
            }
        }

        THREAD_ID_POOL.lock().dealloc(self.id);
    }
}

impl ThreadHandle {
    pub unsafe fn release(&self, addr: usize) {
        if let Some(dtorlist) = self.0.upgrade() {
            if let Some(dtor) = dtorlist.lock().remove(&addr) {
                dtor.drop();
            }
        }
    }
}

#[inline]
pub fn get() -> usize {
    THREAD_STATE.with(|state| state.id)
}

pub unsafe fn push<T: 'static>(addr: usize, ptr: NonNull<Option<T>>) -> ThreadHandle {
    let dtor = Dtor::new(ptr);

    THREAD_STATE.with(|state| {
        state.list.lock().insert(addr, dtor);
        ThreadHandle(Arc::downgrade(&state.list))
    })
}
