//! Efficient per-object thread-local storage implementation.
//!
//! ```rust
//! use std::thread;
//! use std::sync::Arc;
//! use std::cell::RefCell;
//! use per_thread_object::ThreadLocal;
//!
//! fn default() -> RefCell<u32> {
//!     RefCell::new(0x0)
//! }
//!
//! let tl: Arc<ThreadLocal<RefCell<u32>>> = Arc::new(ThreadLocal::new());
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

#[cfg(not(feature = "loom"))]
mod loom;

#[cfg(feature = "loom")]
use loom;

mod thread;
mod page;

use std::ptr::NonNull;
use page::Storage;

pub use page::DEFAULT_PAGE_CAP;


/// Per-object thread-local storage
///
/// ## Capacity
///
/// `per-thread-object` has no max capacity limit,
/// each `ThreadLocal` instance will create its own memory space
/// instead of using global space.
///
/// this crate supports any number of threads,
/// but only the [DEFAULT_PAGE_CAP] threads are lock-free.
///
/// ## Panic when dropping
///
/// `ThreadLocal` will release object at the end of thread.
/// If panic occurs during this process, it may cause a memory leak.
pub struct ThreadLocal<T: Send + 'static> {
    pool: Storage<T>
}

impl<T: Send + 'static> ThreadLocal<T> {
    pub fn new() -> ThreadLocal<T> {
        ThreadLocal {
            pool: Storage::new()
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
        use std::convert::Infallible;

        match self.get_or_try::<_, Infallible>(|| Ok(f())) {
            Ok(val) => val,
            Err(never) => match never {}
        }
    }

    #[inline]
    pub fn get_or_try<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>
    {
        let id = thread::get();
        let obj = unsafe { self.pool.get_or_new(id) };

        let val = match obj {
            Some(val) => val,
            None => {
                let ptr = NonNull::from(&*obj);
                let val = obj.get_or_insert(f()?);

                ThreadLocal::or_try(&self.pool, id, ptr);

                val
            }
        };

        Ok(val)
    }

    #[cold]
    fn or_try(pool: &Storage<T>, id: usize, ptr: NonNull<Option<T>>) {
        let thread_handle = unsafe {
            thread::push(pool.as_threads_ref(), ptr)
        };

        pool.insert_thread_handle(id, thread_handle);
    }
}

impl<T: Send + 'static> Default for ThreadLocal<T> {
    #[inline]
    fn default() -> ThreadLocal<T> {
        ThreadLocal::new()
    }
}

unsafe impl<T: Send> Send for ThreadLocal<T> {}
unsafe impl<T: Send> Sync for ThreadLocal<T> {}
