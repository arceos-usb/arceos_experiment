// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(test), no_std)]
#![allow(clippy::type_repetition_in_bounds)]
#![feature(strict_provenance)]
#![feature(allocator_api)]
use core::{
    alloc::{Allocator, Layout},
    convert::{TryFrom, TryInto},
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

use axhal::mem::{virt_to_phys, PhysAddr, VirtAddr};
use page_table::PageSize;

pub struct PageBox<T: ?Sized> {
    virt: VirtAddr,
    bytes: usize,
    _marker: PhantomData<T>,
}
impl<T> PageBox<T> {
    fn write_initial_value(&mut self, x: T) {
        // SAFETY: This operation is safe because the memory `self.virt.as_mut_ptr` points is
        // allocated, and is page-aligned.
        unsafe {
            ptr::write(self.virt.as_mut_ptr() as *mut T, x);
        }
    }
}
impl<T> Deref for PageBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: This operation is safe because the memory region `virt` points is allocated and
        // is not used by the others.
        unsafe { &*(self.virt.as_ptr() as *mut T) }
    }
}
impl<T> DerefMut for PageBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: This operation is safe because the memory region `virt` points is allocated and
        // is not used by the others.
        unsafe { &mut *(self.virt.as_mut_ptr() as *mut T) }
    }
}
impl<T: fmt::Display> fmt::Display for PageBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T> fmt::Debug for PageBox<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T> Default for PageBox<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::from(T::default())
    }
}
impl<T> Clone for PageBox<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::from((**self).clone())
    }
}
impl<T> From<T> for PageBox<T> {
    fn from(x: T) -> Self {
        assert_alignment(&x);
        let bytes = mem::size_of::<T>();
        let mut page_box = Self::from_bytes(bytes);
        page_box.write_initial_value(x);
        page_box
    }
}

impl<T> PageBox<[T]>
where
    T: Clone,
{
    pub fn new_slice(x: T, num_of_elements: usize) -> Self {
        assert_alignment(&x);
        let bytes = mem::size_of::<T>() * num_of_elements;
        let mut page_box = Self::from_bytes(bytes);
        page_box.write_all_elements_with_same_value(x);
        page_box
    }

    pub fn alloc_pages(pages: usize, zero: T) -> Self {
        let bytes = PageSize::Size4K as usize * pages;
        let mut page_box = Self::from_bytes(bytes);
        page_box.write_all_elements_with_same_value(zero);
        page_box
    }

    fn write_all_elements_with_same_value(&mut self, x: T) {
        for i in 0..self.len() {
            let ptr: usize =
                usize::try_from(self.virt.as_usize()).unwrap() + mem::size_of::<T>() * i;

            // SAFETY: This operation is safe. The memory ptr points is allocated and is aligned
            // because the first elements is page-aligned.
            unsafe { ptr::write(ptr as *mut T, x.clone()) }
        }
    }

    fn num_of_elements(&self) -> usize {
        self.bytes / mem::size_of::<T>()
    }
}

impl<T> Deref for PageBox<[T]>
where
    T: Clone,
{
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.virt.as_ptr() as *mut T, self.num_of_elements()) }
    }
}
impl<T> DerefMut for PageBox<[T]>
where
    T: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            slice::from_raw_parts_mut(self.virt.as_mut_ptr() as *mut T, self.num_of_elements())
        }
    }
}
impl<T> fmt::Display for PageBox<[T]>
where
    T: Clone,
    [T]: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T> fmt::Debug for PageBox<[T]>
where
    T: Clone,
    [T]: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T> Clone for PageBox<[T]>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut b = Self::new_slice(self[0].clone(), self.len());

        for (dst, src) in b.iter_mut().zip(self.iter()) {
            *dst = src.clone();
        }

        b
    }
}
impl<T: Copy> From<&[T]> for PageBox<[T]> {
    fn from(s: &[T]) -> Self {
        let b = Self::new_slice(s[0], s.len());

        unsafe {
            ptr::copy_nonoverlapping(s.as_ptr(), b.virt_addr().as_mut_ptr() as *mut T, s.len());
        }

        b
    }
}

impl<T: ?Sized> PageBox<T> {
    #[must_use]
    pub fn virt_addr(&self) -> VirtAddr {
        self.virt
    }

    /// # Panics
    ///
    /// This method panics if the `PageBox` is not mapped.
    #[must_use]
    pub fn phys_addr(&self) -> PhysAddr {
        // let a = virt_to_phys(self.virt);

        // if a.is_null() {
        //     unreachable!("Address: {:?} is not mapped.", self.virt);
        // }

        // a
        PhysAddr::from(self.virt.as_usize())
    }

    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    fn from_bytes(bytes: usize) -> Self {
        let alloc_pages = VirtAddr::from(
            axalloc::global_no_cache_allocator()
                .allocate(Layout::from_size_align(bytes, PageSize::Size4K as usize).unwrap())
                .expect("error on alloc page")
                .addr()
                .get(),
        );
        //TODO

        // assert!(!virt.is_null(), "Failed to allocate pages.");

        Self {
            virt: alloc_pages,
            bytes,
            _marker: PhantomData,
        }
    }
}
impl<T: ?Sized> Drop for PageBox<T> {
    fn drop(&mut self) {
        let num_of_pages = as_num_of_pages(self.bytes);

        axalloc::global_allocator().dealloc_pages(self.virt.as_usize(), num_of_pages);
    }
}

#[cfg(test)]
mod tests {
    use super::PageBox;

    #[test]
    fn clone_single_element() {
        let b = PageBox::from(3);
        let b2 = b.clone();

        assert_eq!(*b, *b2);
    }

    #[test]
    fn clone_slice() {
        let b = PageBox::new_slice(334, 5);
        let b2 = b.clone();

        assert_eq!(*b, *b2);
    }

    #[test]
    fn from_slice() {
        let s: &[i32] = &[3, 3, 4];
        let b = PageBox::<[i32]>::from(s);

        assert_eq!(*b, *s);
    }
}

fn assert_alignment<T>(x: &T) {
    assert!(
        mem::align_of_val(x) <= PageSize::Size4K as usize,
        "The minimum alignment must be less than or equal to 4096."
    );
}

fn as_num_of_pages(usize: usize) -> usize {
    (usize + PageSize::Size4K as usize - 1) / PageSize::Size4K as usize
}
