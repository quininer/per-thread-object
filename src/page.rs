use std::cell::UnsafeCell;
use parking_lot::Mutex;


pub(crate) const PAGE_CAP: usize = 64;

pub struct PagePool<T> {
    page: Page<T>,
    fallback_page: Mutex<Vec<Page<T>>>
}

struct Page<T> {
    ptr: Box<[UnsafeCell<Option<T>>; PAGE_CAP]>
}

impl<T> PagePool<T> {
    pub fn new() -> PagePool<T> {
        PagePool {
            page: Page::new(),
            fallback_page: Mutex::new(Vec::new())
        }
    }

    pub fn get(&self, id: usize) -> Option<&T> {
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

    pub fn get_or_new(&self, id: usize) -> *mut Option<T> {
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
            ( $e:expr ; x64 ) => {
                [
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

        let page = Box::new(arr![UnsafeCell::new(None); x64]);

        Page { ptr: page }
    }

    fn get(&self, index: usize) -> *mut Option<T> {
        unsafe {
            self.ptr.get_unchecked(index).get()
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
