//! Efficient per-object thread-local storage implementation.
//!
//! ```rust
//! use std::thread;
//! use std::cell::RefCell;
//! use per_thread_object::ThreadLocal;
//!
//! fn default() -> RefCell<u32> {
//!     RefCell::new(0x0)
//! }
//!
//! let tl: ThreadLocal<RefCell<u32>> = ThreadLocal::new();
//! let tl2 = tl.clone();
//!
//! thread::spawn(move || {
//!     *tl2.get_or(default)
//!         .borrow_mut() += 1;
//!     assert_eq!(0x1, *tl2.get().unwrap().borrow());
//! })
//!     .join()
//!     .unwrap();
//!
//! *tl.get_or(default)
//!     .borrow_mut() += 2;
//! assert_eq!(0x2, *tl.get_or(default).borrow());
//! ```

mod rc;
mod thread;
mod page;

use rc::HeapRc;
use page::PagePool;


/// Per-object thread-local storage
///
/// ## Cloneable
///
/// `ThreadLocal` uses built-in reference counting,
/// so it is usually not necessary to use `Arc`.
///
/// ## Capacity
///
/// `per-thread-object` has no capacity limit,
/// each `ThreadLocal` instance will create its own memory space
/// instead of using global space.
///
/// this crate supports any number of threads,
/// but only the 64 threads are lock-free.
///
/// ## Panic when dropping
///
/// `ThreadLocal` will release object when calling `clean` or the end of thread.
/// If panic occurs during this process, it may cause a memory leak.
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

    #[inline]
    pub fn get(&self) -> Option<&T> {
        self.pool.get(thread::get())
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

    /// Clean up the objects of this thread.
    ///
    /// Note that this is an `O(n)` operation,
    /// where `n` is all `ThreadLocal` objects in the current thread.
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
