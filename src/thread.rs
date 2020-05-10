use std::cell::RefCell;
use parking_lot::{ lock_api::RawMutex as _, Mutex, RawMutex };
use crate::rc::{ HeapRc, DropRc };
use crate::page::{ PAGE_CAP, PagePool };


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
    list: RefCell<Vec<Dtor>>
}

struct Dtor {
    rc: DropRc,
    ptr: *mut (),
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
    fn new<T: 'static>(rc: DropRc, ptr: *mut Option<T>) -> Dtor {
        unsafe fn try_drop<T: 'static>(ptr: *mut ()) {
            let obj = &mut *ptr.cast::<Option<T>>();
            obj.take();
        }

        Dtor {
            rc,
            ptr: ptr as *mut (),
            drop: try_drop::<T>
        }
    }

    unsafe fn drop(&self) {
        (self.drop)(self.ptr)
    }
}

impl Drop for ThreadState {
    fn drop(&mut self) {
        let list = self.list.borrow_mut();

        for dtor in &*list {
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

pub unsafe fn push<T: 'static>(pool: HeapRc<PagePool<T>>, ptr: *mut Option<T>) {
    let rc = pool.into_droprc();
    let dtor = Dtor::new(rc, ptr);

    THREAD_STATE.with(|state| {
        state.list.borrow_mut().push(dtor);
    });
}

pub unsafe fn clean(addr: *const ()) {
    THREAD_STATE.with(|state| {
        let mut list = state.list.borrow_mut();

        list.retain(|dtor| if dtor.rc.as_ptr() == addr {
            dtor.drop();

            false
        } else {
            true
        })
    });
}
