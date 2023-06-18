use std::{ ops, mem, alloc, slice };
use std::marker::PhantomData;
use std::ptr::{ self, NonNull };


pub struct BoxTail<T, S>(NonNull<Inner<T, S>>);

struct Inner<T, S> {
    value: T,
    arr_len: usize,
    _arr: IncompleteArrayField<S>
}

// Guaranteed alignment
struct IncompleteArrayField<T>(PhantomData<T>, [T; 0]);

impl<T, S> BoxTail<T, S> {
    pub fn new(
        value: T,
        arr_len: usize,
        arr_init: fn(*mut S)
    ) -> Self {
        // dont handle drop, because we do not need
        assert!(!mem::needs_drop::<S>());

        let layout = alloc::Layout::new::<Inner<T, S>>();
        let array_layout = alloc::Layout::array::<S>(arr_len).unwrap();
        let (layout, _offset) = layout.extend(array_layout).unwrap();

        unsafe {
            let ptr = NonNull::new(alloc::alloc(layout).cast::<Inner<T, S>>()).unwrap();

            ptr.as_ptr().write(Inner {
                value, arr_len,
                _arr: IncompleteArrayField(PhantomData, [])
            });

            // dont use `IncompleteArrayField` to take pointer because miri is not happy
            let arr_ptr = ptr.as_ptr().add(1).cast::<S>();

            for idx in 0..arr_len {
                let elem = arr_ptr.add(idx);
                arr_init(elem);
            }

            BoxTail(ptr)
        }
    }

    #[inline]
    pub fn array_len(&self) -> usize {
        unsafe {
            self.0.as_ref().arr_len
        }
    }

    #[inline]
    pub fn array(&self) -> &[S] {
        unsafe {
            let arr_len = self.0.as_ref().arr_len;
            slice::from_raw_parts(self.0.as_ptr().add(1).cast::<S>(), arr_len)
        }
    }
}

impl<T, S> ops::Deref for BoxTail<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &self.0.as_ref().value
        }
    }
}

impl<T, S> ops::DerefMut for BoxTail<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut self.0.as_mut().value
        }
    }
}

impl<T, S> Drop for BoxTail<T, S> {
    fn drop(&mut self) {
        unsafe {
            if mem::needs_drop::<T>() {
                ptr::drop_in_place(self.0.as_ptr());
            }

            let arr_len = self.0.as_ref().arr_len;
            let layout = alloc::Layout::new::<Inner<T, S>>();
            let array_layout = alloc::Layout::array::<S>(arr_len).unwrap();
            let (layout, _offset) = layout.extend(array_layout).unwrap();
            alloc::dealloc(self.0.as_ptr().cast(), layout);
        }
    }
}
