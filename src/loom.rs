pub mod sync {
    pub use std::sync::Arc;

    pub struct Mutex<T>(parking_lot::Mutex<T>);

    impl<T> Mutex<T> {
        #[inline]
        pub const fn new(t: T) -> Mutex<T> {
            use parking_lot::lock_api::RawMutex;

            Mutex(parking_lot::Mutex::const_new(parking_lot::RawMutex::INIT, t))
        }

        #[inline]
        pub fn lock(&self) -> Result<parking_lot::MutexGuard<'_, T>, std::convert::Infallible> {
            Ok(self.0.lock())
        }
    }
}

pub mod cell {
    pub struct UnsafeCell<T>(std::cell::UnsafeCell<T>);

    impl<T> UnsafeCell<T> {
        #[inline]
        pub fn new(t: T) -> UnsafeCell<T> {
            UnsafeCell(std::cell::UnsafeCell::new(t))
        }

        #[inline]
        pub fn with<F, R>(&self, f: F) -> R
        where F: FnOnce(*const T) -> R
        {
            f(self.0.get())
        }

        #[inline]
        pub fn with_mut<F, R>(&self, f: F) -> R
        where F: FnOnce(*mut T) -> R
        {
            f(self.0.get())
        }
    }
}
