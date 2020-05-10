use std::cmp;
use std::ptr::NonNull;
use std::borrow::Borrow;


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

impl PartialEq for DropRc {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr.eq(&other.ptr)
    }
}

impl PartialOrd for DropRc {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.ptr.partial_cmp(&other.ptr)
    }
}

impl Eq for DropRc {}

impl Ord for DropRc {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

impl Borrow<NonNull<()>> for DropRc {
    #[inline]
    fn borrow(&self) -> &NonNull<()> {
        &self.ptr
    }
}

impl Drop for DropRc {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr.as_ptr());
        }
    }
}
