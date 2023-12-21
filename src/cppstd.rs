/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           cppstd.rs
 *  DESCRIPTION:    C++ structs
 *  COPYRIGHT:      (c) 2023 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use std::ffi::{c_char, CStr};

#[repr(C)]
pub struct StdVector<T> {
    first: *const T,
    last: *const T,
    end: *const T,
}

impl<T> StdVector<T> {
    pub fn len(&self) -> usize {
        (self.last as usize - self.first as usize) / std::mem::size_of::<T>()
    }

    pub fn capacity(&self) -> usize {
        (self.end as usize - self.first as usize) / std::mem::size_of::<T>()
    }
}

impl<'a, T> IntoIterator for &'a StdVector<T> {
    type Item = &'a T;
    type IntoIter = StdVectorIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        StdVectorIterator {
            current: self.first,
            end: self.last,
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct StdVectorIterator<'a, T> {
    current: *const T,
    end: *const T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for StdVectorIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current != self.end {
            let value = unsafe { &*self.current };
            self.current = unsafe { self.current.offset(1) };
            Some(value)
        } else {
            None
        }
    }
}

#[repr(C)]
union StdStringUnion {
    buf: [u8; 16],
    ptr: *const c_char,
}

#[repr(C)]
pub struct StdString {
    str: StdStringUnion,
    size: u32,
    capacity: u32,
}

impl StdString {
    pub fn to_string(&self) -> String {
        unsafe {
            if self.size < 16 {
                CStr::from_bytes_until_nul(&self.str.buf).unwrap().to_string_lossy().to_string()
            } else {
                CStr::from_ptr(self.str.ptr).to_string_lossy().to_string()
            }
        }
    }
}
