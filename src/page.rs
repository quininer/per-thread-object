use std::ptr::NonNull;
use std::mem::ManuallyDrop;
use parking_lot::Mutex;


pub(crate) const PAGE_CAP: usize = 128;

pub struct PagePool<T> {
    page: Page<T>,
    fallback_page: Mutex<Vec<Page<T>>>
}

struct Page<T> {
    ptr: NonNull<[ManuallyDrop<Option<T>>; PAGE_CAP]>
}

impl<T> PagePool<T> {
    pub fn new() -> PagePool<T> {
        PagePool {
            page: Page::new(),
            fallback_page: Mutex::new(Vec::new())
        }
    }

    pub fn find(&self, id: usize) -> Option<&T> {
        let (page_id, index) = map_index(id);

        let obj = if page_id == 0 {
            self.page.get(index)
        } else {
            let pages = self.fallback_page.lock();
            pages.get(page_id - 1)?.get(index)
        };

        unsafe {
            (&*obj).as_ref()
        }
    }

    pub fn insert(&self, id: usize) -> *mut Option<T> {
        let (page_id, index) = map_index(id);

        if page_id == 0 {
            self.page.get(index)
        } else {
            let mut pages = self.fallback_page.lock();
            let page_id = page_id - 1;

            if page_id > pages.len() {
                pages.resize_with(page_id + 1, Page::new);
            }

            pages[page_id].get(index)
        }
    }
}

impl<T> Page<T> {
    fn new() -> Page<T> {
        macro_rules! arr {
            ( $e:expr ; x128 ) => {
                [
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,

                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,
                    $e, $e, $e, $e, $e, $e, $e, $e,

                ]
            }
        }

        let page = Box::new(arr![ManuallyDrop::new(None); x128]);
        let ptr = Box::leak(page).into();

        Page { ptr }
    }

    fn get(&self, index: usize) -> *mut Option<T> {
        unsafe {
            let array: &mut [ManuallyDrop<Option<T>>; PAGE_CAP] = &mut *self.ptr.as_ptr();
            array
                .as_mut_ptr()
                .add(index)
                .cast()
        }
    }
}

impl<T> Drop for Page<T> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.ptr.as_ptr());
        }
    }
}

fn map_index(n: usize) -> (usize, usize) {
    if n <= PAGE_CAP {
        (0, n)
    } else {
        let i = n / PAGE_CAP;
        (i, n - (i * PAGE_CAP))
    }
}
