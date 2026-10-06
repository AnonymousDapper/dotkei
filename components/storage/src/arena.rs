// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#![allow(clippy::mut_from_ref)]

use std::alloc::Layout;
use std::cell::{Cell, RefCell};
use std::mem::{self, MaybeUninit};
use std::ptr::{self, NonNull};
use std::{cmp, hint, slice};

use smallvec::SmallVec;

const PAGE: usize = 4096;
const HUGE_PAGE: usize = 2 * 1024 * 1024;

const DROPLESS_ALIGNMENT: usize = align_of::<usize>();

#[inline(always)]
fn align_down(val: usize, align: usize) -> usize {
    val & !(align - 1)
}

#[inline(always)]
fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

#[inline(never)]
#[cold]
fn outline<F: FnOnce() -> R, R>(f: F) -> R {
    f()
}

struct ArenaChunk<T = u8> {
    storage: NonNull<[MaybeUninit<T>]>,
    entries: usize,
}

impl<T> ArenaChunk<T> {
    #[inline]
    unsafe fn new(capacity: usize) -> ArenaChunk<T> {
        ArenaChunk {
            storage: NonNull::from(Box::leak(Box::new_uninit_slice(capacity))),
            entries: 0,
        }
    }

    #[inline]
    unsafe fn destroy(&mut self, len: usize) {
        if mem::needs_drop::<T>() {
            unsafe {
                let slice = self.storage.as_mut();
                slice[..len].assume_init_drop();
            }
        }
    }

    #[inline]
    fn start(&mut self) -> *mut T {
        self.storage.as_ptr() as *mut T
    }

    #[inline]
    fn end(&mut self) -> *mut T {
        unsafe {
            if size_of::<T>() == 0 {
                ptr::without_provenance_mut(!0)
            } else {
                self.start().add(self.storage.len())
            }
        }
    }
}

pub struct DroplessArena {
    start: Cell<*mut u8>,
    end: Cell<*mut u8>,

    chunks: RefCell<Vec<ArenaChunk>>,
}

impl Default for DroplessArena {
    #[inline]
    fn default() -> DroplessArena {
        DroplessArena {
            start: Cell::new(ptr::null_mut()),
            end: Cell::new(ptr::null_mut()),
            chunks: Default::default(),
        }
    }
}

impl DroplessArena {
    #[inline(never)]
    #[cold]
    fn grow(&self, layout: Layout) {
        let additional = layout.size() + cmp::max(DROPLESS_ALIGNMENT, layout.align()) - 1;

        unsafe {
            let mut chunks = self.chunks.borrow_mut();
            let mut new_cap;

            if let Some(last_chunk) = chunks.last_mut() {
                new_cap = last_chunk.storage.len().min(HUGE_PAGE / 2);
                new_cap *= 2;
            } else {
                new_cap = PAGE;
            }

            new_cap = cmp::max(additional, new_cap);

            chunks.push(ArenaChunk::new(align_up(new_cap, PAGE)));

            let chunk = chunks.last_mut().unwrap();
            //let chunk = chunks.push_mut(ArenaChunk::new(align_up(new_cap, PAGE)));
            self.start.set(chunk.start());

            let end = align_down(chunk.end().addr(), DROPLESS_ALIGNMENT);

            debug_assert!(chunk.start().addr() <= end);

            self.end.set(chunk.end().with_addr(end));
        }
    }

    #[inline]
    pub fn alloc_raw(&self, layout: Layout) -> *mut u8 {
        assert!(layout.size() != 0);

        loop {
            let start = self.start.get().addr();
            let old_end = self.end.get();
            let end = old_end.addr();

            let bytes = align_up(layout.size(), DROPLESS_ALIGNMENT);

            unsafe { hint::assert_unchecked(end == align_down(end, DROPLESS_ALIGNMENT)) };

            if let Some(sub) = end.checked_sub(bytes) {
                let new_end = align_down(sub, layout.align());
                if start <= new_end {
                    let new_end = old_end.with_addr(new_end);
                    self.end.set(new_end);
                    return new_end;
                }
            }

            self.grow(layout);
        }
    }

    #[inline]
    pub fn alloc<T>(&self, object: T) -> &mut T {
        assert!(!mem::needs_drop::<T>());
        assert!(size_of::<T>() != 0);

        let mem = self.alloc_raw(Layout::new::<T>()) as *mut T;

        unsafe {
            ptr::write(mem, object);
            &mut *mem
        }
    }

    #[inline]
    pub fn alloc_slice<T>(&self, slice: &[T]) -> &mut [T]
    where
        T: Copy,
    {
        assert!(!mem::needs_drop::<T>());
        assert!(size_of::<T>() != 0);
        assert!(!slice.is_empty());

        let mem = self.alloc_raw(Layout::for_value::<[T]>(slice)) as *mut T;

        unsafe {
            mem.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
            slice::from_raw_parts_mut(mem, slice.len())
        }
    }

    #[inline]
    pub fn alloc_str(&self, string: &str) -> &str {
        let slice = self.alloc_slice(string.as_bytes());

        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    #[inline]
    unsafe fn write_from_iter<T, I: Iterator<Item = T>>(
        &self,
        mut iter: I,
        len: usize,
        mem: *mut T,
    ) -> &mut [T] {
        let mut i = 0;

        loop {
            unsafe {
                match iter.next() {
                    Some(value) if i < len => mem.add(i).write(value),
                    Some(_) | None => {
                        return slice::from_raw_parts_mut(mem, i);
                    }
                }
            }

            i += 1;
        }
    }

    #[inline]
    pub fn alloc_from_iter<T, I: IntoIterator<Item = T>>(&self, iter: I) -> &mut [T] {
        let iter = iter.into_iter();

        assert!(size_of::<T>() != 0);
        assert!(!mem::needs_drop::<T>());

        let size_hint = iter.size_hint();

        match size_hint {
            (min, Some(max)) if min == max => {
                let len = min;

                if len == 0 {
                    return &mut [];
                }

                let mem = self.alloc_raw(Layout::array::<T>(len).unwrap()) as *mut T;

                unsafe { self.write_from_iter(iter, len, mem) }
            }
            (_, _) => outline(move || {
                self.try_alloc_from_iter(iter.map(Ok::<T, ()>))
                    .expect("try_alloc failed")
            }),
        }
    }

    #[inline]
    pub fn try_alloc_from_iter<T, E>(
        &self,
        iter: impl IntoIterator<Item = Result<T, E>>,
    ) -> Result<&mut [T], E> {
        assert!(size_of::<T>() != 0);

        let vec: Result<SmallVec<[T; 8]>, E> = iter.into_iter().collect();
        let mut vec = vec?;

        if vec.is_empty() {
            return Ok(&mut []);
        }

        let len = vec.len();
        Ok(unsafe {
            let start_ptr = self.alloc_raw(Layout::for_value::<[T]>(vec.as_slice())) as *mut T;
            vec.as_ptr().copy_to_nonoverlapping(start_ptr, len);
            vec.set_len(0);
            slice::from_raw_parts_mut(start_ptr, len)
        })
    }
}
