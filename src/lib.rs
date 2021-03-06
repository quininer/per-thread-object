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
//!     tl2.with_or(|val| *val.borrow_mut() += 1, default);
//!     let val = tl2.with(|val| *val.borrow());
//!     assert_eq!(0x1, val);
//! })
//!     .join()
//!     .unwrap();
//!
//! tl.with_or(|val| *val.borrow_mut() += 2, default);
//! assert_eq!(0x2, tl.with_or(|val| *val.borrow(), default));
//! ```

#[cfg(not(feature = "loom"))]
mod loom;

#[cfg(feature = "loom")]
use loom;

mod thread;
mod page;

use std::ptr::NonNull;
use loom::cell::UnsafeCell;
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
    pub fn with<F, R>(&self, f: F)
        -> R
    where
        F: FnOnce(&T) -> R
    {
        let val = unsafe {
            self.pool.get(thread::get()).expect("Uninitialized")
        };
        f(val)
    }

    #[inline]
    pub fn try_with<F, R>(&self, f: F)
        -> Option<R>
    where
        F: FnOnce(&T) -> R
    {
        let val = unsafe {
            self.pool.get(thread::get())?
        };
        Some(f(val))
    }

    #[inline]
    pub fn with_or<F, I, R>(&self, f: F, init: I)
        -> R
    where
        F: FnOnce(&T) -> R,
        I: FnOnce() -> T
    {
        use std::convert::Infallible;

        match self.with_try_or::<_, _, _, Infallible>(f, || Ok(init())) {
            Ok(val) => val,
            Err(never) => match never {}
        }
    }

    #[inline]
    pub fn with_try_or<F, I, R, E>(&self, f: F, init: I)
        -> Result<R, E>
    where
        F: FnOnce(&T) -> R,
        I: FnOnce() -> Result<T, E>
    {
        let id = thread::get();
        let ptr = unsafe { self.pool.get_or_new(id) };

        let obj = unsafe { &*ptr.as_ptr() };
        let val = if let Some(val) = obj.with(|val| unsafe { &*val }) {
            val
        } else {
            let val = obj.with_mut(|val| {
                let val = unsafe { &mut *val }.get_or_insert(init()?);
                Ok(val)
            })?;

            ThreadLocal::or_try(&self.pool, id, ptr);

            val
        };

        Ok(f(val))
    }

    #[cold]
    fn or_try(pool: &Storage<T>, id: usize, ptr: NonNull<UnsafeCell<Option<T>>>) {
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
