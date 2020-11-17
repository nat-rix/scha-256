use core::mem::{ManuallyDrop, MaybeUninit};
use std::fmt::Debug;

#[derive(Clone)]
pub struct List<T: Sized, const N: usize> {
    data: ManuallyDrop<[T; N]>,
    size: usize,
}

impl<T, const N: usize> Default for List<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> List<T, N> {
    pub fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            size: 0,
        }
    }

    pub fn from_array<const M: usize>(arr: [T; M]) -> Self {
        let mut s = Self::new();
        s.extend(arr);
        s
    }

    pub fn extend<const M: usize>(&mut self, arr: [T; M]) {
        for v in core::array::IntoIter::new(arr) {
            self.append(v)
        }
    }

    pub fn append(&mut self, item: T) {
        self.data[self.size] = item;
        self.size += 1
    }

    pub fn slice(&self) -> &[T] {
        &self.data[..self.size]
    }

    pub fn slice_mut(&mut self) -> &mut [T] {
        &mut self.data[..self.size]
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.explicit_drop();
        self.size = 0
    }

    fn explicit_drop(&mut self) {
        let iter = unsafe { ManuallyDrop::take(&mut self.data) };
        for item in core::array::IntoIter::new(iter).skip(self.size) {
            core::mem::forget(item)
        }
    }
}

impl<T: PartialEq + 'static, const N: usize> List<T, N> {
    pub fn swap_remove(&mut self, item: &T) {
        let mut unsafeself = || unsafe { &mut *(self as *mut Self) };
        if let Some(item) = unsafeself().slice_mut().iter_mut().find(|i| i == &item) {
            if let Some(last) = unsafeself().slice_mut().last_mut() {
                unsafe { (item as *mut T).swap(last) };
                self.size -= 1;
            }
        }
    }

    pub fn swap_remove_at(&mut self, n: usize) {
        let sz = self.size;
        if n + 1 < self.size {
            self.slice_mut().swap(sz - 1, n)
        }
        self.size -= 1;
    }

    pub fn filter<F: FnMut(&T) -> bool>(&mut self, start: usize, mut f: F) {
        for i in (start..self.size).rev() {
            if !f(&self.slice()[i]) {
                if i + 1 < self.size {
                    let n = self.size;
                    self.slice_mut().swap(n - 1, i)
                }
                self.size -= 1
            }
        }
    }
}

impl<T: Sized, const N: usize> Drop for List<T, N> {
    fn drop(&mut self) {
        self.explicit_drop()
    }
}

impl<T: Sized + Debug, const N: usize> Debug for List<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.slice()).finish()
    }
}

pub fn array_from_fn<T: Sized, F: FnMut() -> T, const N: usize>(mut f: F) -> [T; N] {
    let mut arr: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
    for v in &mut arr[..] {
        *v = MaybeUninit::new(f())
    }
    unsafe { core::mem::transmute_copy(&arr) }
}
