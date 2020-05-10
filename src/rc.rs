use std::ptr::NonNull;


pub struct DropRc {
    ptr: NonNull<()>,
    drop: unsafe fn(*mut ())
}

impl DropRc {
    #[inline]
    pub unsafe fn new(ptr: NonNull<()>, drop: unsafe fn(*mut ())) -> DropRc {
        DropRc { ptr, drop }
    }
}

impl Drop for DropRc {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr.as_ptr());
        }
    }
}
