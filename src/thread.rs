use std::ptr::NonNull;
use std::cell::RefCell;
use parking_lot::{ lock_api::RawMutex as _, Mutex, RawMutex };
use crate::rc::DropRc;
use crate::page::PAGE_CAP;


static THREAD_ID_POOL: Mutex<ThreadIdPool> =
    Mutex::const_new(RawMutex::INIT, ThreadIdPool::new());

thread_local!{
    static THREAD_STATE: ThreadState = ThreadState::new();
}

struct ThreadIdPool {
    max: usize,
    small_freelist: Vec<usize>,
    slow_freelist: Vec<usize>
}

struct ThreadState {
    id: usize,
    list: RefCell<Vec<(DropRc, Dtor)>>
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
            id
        } else {
            let id = self.max;
            self.max = id.checked_add(1).expect("thread id overflow");
            id
        }
    }

    fn dealloc(&mut self, id: usize) {
        if id <= PAGE_CAP {
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
            list: RefCell::new(Vec::new())
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
        let list = self.list.borrow_mut();

        for (_, dtor) in &*list {
            unsafe {
                dtor.drop();
            }
        }

        THREAD_ID_POOL.lock().dealloc(self.id);
    }
}

#[inline]
pub fn get() -> usize {
    THREAD_STATE.with(|state| state.id)
}

pub unsafe fn push<T: 'static>(rc: DropRc, ptr: NonNull<Option<T>>) {
    let dtor = Dtor::new(ptr);

    THREAD_STATE.with(|state| state.list.borrow_mut().push((rc, dtor)));
}
