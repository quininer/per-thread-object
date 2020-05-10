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

use std::ptr::NonNull;
use page::Pages;


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
/// but only the N threads are lock-free.
///
/// ## Panic when dropping
///
/// `ThreadLocal` will release object when calling `clean` or the end of thread.
/// If panic occurs during this process, it may cause a memory leak.
#[derive(Clone)]
pub struct ThreadLocal<T: 'static> {
    pool: Pages<T>
}

impl<T: 'static> ThreadLocal<T> {
    pub fn new() -> ThreadLocal<T> {
        ThreadLocal {
            pool: Pages::new()
        }
    }

    #[inline]
    pub fn get(&self) -> Option<&T> {
        unsafe {
            self.pool.get(thread::get())
        }
    }

    #[inline]
    pub fn get_or<F: FnOnce() -> T>(&self, f: F) -> &T {
        enum Never {}

        match self.get_or_try::<_, Never>(|| Ok(f())) {
            Ok(val) => val,
            Err(never) => match never {}
        }
    }

    pub fn get_or_try<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>
    {
        let obj = unsafe { self.pool.get_or_new(thread::get()) };

        let val = match obj {
            Some(val) => val,
            None => {
                let ptr = NonNull::from(&*obj);
                let val = obj.get_or_insert(f()?);
                let pool = self.pool.clone();

                unsafe {
                    thread::push(pool.into_droprc(), ptr);
                }

                val
            }
        };

        Ok(val)
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
