mod rc;
mod thread;
mod page;

use rc::HeapRc;
use page::PagePool;


#[derive(Clone)]
pub struct ThreadLocal<T: 'static> {
    pool: HeapRc<PagePool<T>>
}

impl<T: 'static> ThreadLocal<T> {
    pub fn new() -> ThreadLocal<T> {
        ThreadLocal {
            pool: HeapRc::new(PagePool::new())
        }
    }

    pub fn get(&self) -> Option<&T> {
        let id = thread::get();

        self.pool.get(id)
    }

    pub fn get_or<F: FnOnce() -> T>(&self, f: F) -> &T {
        let id = thread::get();

        let obj = unsafe { &mut *self.pool.get_or_new(id) };

        match obj {
            Some(val) => val,
            None => {
                let pool = self.pool.clone();
                let ptr = obj as *mut _;
                let val = obj.get_or_insert(f());

                unsafe {
                    thread::push(pool, ptr);
                }

                val
            }
        }
    }

    pub fn clean(&self) {
        unsafe {
            thread::clean(self.pool.as_ptr());
        }
    }
}

impl<T: 'static> Default for ThreadLocal<T> {
    #[inline]
    fn default() -> ThreadLocal<T> {
        ThreadLocal::new()
    }
}

unsafe impl<T> Send for ThreadLocal<T> {}
unsafe impl<T> Sync for ThreadLocal<T> {}
