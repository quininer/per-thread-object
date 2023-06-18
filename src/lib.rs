//! Efficient per-object thread-local storage implementation.
//!
//! ```rust
//! # if cfg!(feature = "loom") || cfg!(feature = "shuttle") { return }
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
//!     per_thread_object::stack_token!(token);
//!
//!     *tl2.get_or_init(token, default).borrow_mut() += 1;
//!     let val = *tl2.get(token).unwrap().borrow();
//!     assert_eq!(0x1, val);
//! })
//!     .join()
//!     .unwrap();
//!
//! per_thread_object::stack_token!(token);
//!
//! *tl.get_or_init(token, default).borrow_mut() += 2;
//! assert_eq!(0x2, *tl.get_or_init(token, default).borrow());
//! ```

#[cfg(not(feature = "loom"))]
mod loom;

#[cfg(feature = "loom")]
use loom;

mod util;
mod thread;
mod page;

use std::ptr::NonNull;
use loom::cell::UnsafeCell;
use page::Storage;


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

pub struct StackToken {
    _marker: std::marker::PhantomData<*const ()>,
}

impl StackToken {
    #[doc(hidden)]
    pub unsafe fn __private_new() -> StackToken {
        StackToken {
            _marker: std::marker::PhantomData,
        }
    }
}

#[macro_export]
macro_rules! stack_token {
    ($name:ident) => {
        #[allow(unsafe_code)]
        let $name = &unsafe { $crate::StackToken::__private_new() };
    };
}

impl<T: Send + 'static> ThreadLocal<T> {
    pub fn new() -> ThreadLocal<T> {
        #[cfg(not(feature = "loom"))]
        #[cfg(not(feature = "shuttle"))]
        let default = 16;

        #[cfg(any(feature = "loom", feature = "shuttle"))]
        let default = 3;

        ThreadLocal::with_threads(default)
    }

    pub fn with_threads(num: usize) -> ThreadLocal<T> {
        ThreadLocal {
            pool: Storage::with_threads(num)
        }
    }

    #[inline]
    pub fn get<'stack>(&'stack self, _token: &'stack StackToken) -> Option<&'stack T> {
        unsafe {
            self.pool.get(thread::get())
        }
    }

    #[inline]
    pub fn get_or_init<'stack, F>(&'stack self, token: &'stack StackToken, init: F)
        -> &'stack T
    where
        F: FnOnce() -> T
    {
        use std::convert::Infallible;

        match self.get_or_try_init::<_, Infallible>(token, || Ok(init())) {
            Ok(val) => val,
            Err(err) => match err {}
        }
    }

    #[inline]
    pub fn get_or_try_init<'stack, F, E>(&'stack self, _token: &'stack StackToken, init: F)
        -> Result<&'stack T, E>
    where
        F: FnOnce() -> Result<T, E>
    {
        let id = thread::get();
        let ptr = unsafe { self.pool.get_or_new(id) };

        let obj = unsafe { &*ptr.as_ptr() };
        let val = if let Some(val) = obj.with(|val| unsafe { &*val }) {
            val
        } else {
            let newval = init()?;
            let val = obj.with_mut(|val| unsafe { &mut *val }.get_or_insert(newval));

            ThreadLocal::or_try(&self.pool, id, ptr);

            val
        };

        Ok(val)
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
