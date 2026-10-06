// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use std::hash::BuildHasher;

use structures::hash_table::{Entry, HashTable};
use structures::FxBuildHasher;

use storage::arena::DroplessArena;

pub use symbols::*;

mod symbols;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(SymbolIndex);

impl Symbol {
    pub const fn new(n: u32) -> Self {
        Symbol(SymbolIndex::from_u32(n))
    }

    pub fn intern(str: &str, interner: &mut Interner) -> Self {
        interner.intern_str(str)
    }

    pub fn as_str<'a>(&self, interner: &'a mut Interner) -> &'a str {
        interner.get_str(*self)
    }

    pub fn as_u32(self) -> u32 {
        self.0.as_u32()
    }

    // pub fn is_empty(self) -> bool {
    //     todo!()
    // }
}

pub struct Interner {
    arena: DroplessArena,
    indices: HashTable<(&'static [u8], u32)>,
    byte_strs: Vec<&'static [u8]>,
}

impl Interner {
    fn prefill(init: &[&'static str], extra: &[&'static str]) -> Self {
        let values = init
            .iter()
            .copied()
            .chain(extra.iter().copied())
            .map(|s| s.as_bytes());
        let (size_hint, _) = values.size_hint();

        let mut conflicts: Vec<&[u8]> = Vec::new();

        let mut indices: HashTable<(&'static [u8], u32)> = HashTable::with_capacity(size_hint);
        let hasher = FxBuildHasher;

        let mut byte_strs: Vec<&'static [u8]> = Vec::with_capacity(size_hint);

        for v in values {
            match indices.entry(
                hasher.hash_one(v),
                |&(s, _)| s == v,
                |&(s, _)| hasher.hash_one(s),
            ) {
                Entry::Occupied(v) => conflicts.push(v.get().0),
                Entry::Vacant(view) => {
                    view.insert((v, byte_strs.len() as u32));
                    byte_strs.push(v);
                }
            }
        }

        if !conflicts.is_empty() {
            panic!(
                "duplicate symbols added to interner prefill: {:?}",
                conflicts
            )
        }

        Interner {
            arena: Default::default(),
            indices,
            byte_strs,
        }
    }

    fn intern_str(&mut self, s: &str) -> Symbol {
        Symbol::new(self.intern_inner(s.as_bytes()))
    }

    // fn intern_byte_str(&self, byte_str: &[u8]) -> ByteSymbol {}

    #[inline]
    fn intern_inner(&mut self, byte_str: &[u8]) -> u32 {
        let hasher = FxBuildHasher;
        let input_hash = hasher.hash_one(byte_str);

        match self.indices.entry(
            input_hash,
            |&(s, _)| s == byte_str,
            |&(s, _)| hasher.hash_one(s),
        ) {
            Entry::Occupied(v) => v.get().1,
            Entry::Vacant(view) => {
                let byte_str: &[u8] = self.arena.alloc_slice(byte_str);

                let byte_str: &'static [u8] = unsafe { &*(byte_str as *const [u8]) };
                let idx = self.byte_strs.len() as u32;

                view.insert((byte_str, idx));
                self.byte_strs.push(byte_str);

                idx
            }
        }
    }

    fn get_str(&self, symbol: Symbol) -> &str {
        let byte_str = self.byte_strs[symbol.0.as_usize()];

        unsafe { str::from_utf8_unchecked(byte_str) }
    }

    // fn get_byte_str(&self, symbol: ByteSymbol) -> &[u8] {}
}

#[derive(Clone, Copy)]
struct SymbolIndex {
    __private_index: u32,
}

const INDEX_MAX: u32 = 0xFFFF_FF00;

impl SymbolIndex {
    const MAX_AS_U32: u32 = INDEX_MAX;

    const MAX: Self = Self::from_u32(INDEX_MAX);

    const ZERO: Self = Self::from_u32(0);

    #[inline]
    const fn from_usize(value: usize) -> Self {
        assert!(value <= INDEX_MAX as usize);
        unsafe { Self::from_u32_unchecked(value as u32) }
    }

    #[inline]
    const fn from_u32(value: u32) -> Self {
        assert!(value <= INDEX_MAX);
        unsafe { Self::from_u32_unchecked(value) }
    }

    #[inline]
    const unsafe fn from_u32_unchecked(value: u32) -> Self {
        Self {
            __private_index: value,
        }
    }

    #[inline]
    const fn as_u32(self) -> u32 {
        self.__private_index
    }

    #[inline]
    const fn as_usize(self) -> usize {
        self.as_u32() as usize
    }
}

impl std::ops::Add<usize> for SymbolIndex {
    type Output = Self;
    #[inline]
    fn add(self, other: usize) -> Self {
        Self::from_usize(self.as_usize() + other)
    }
}

impl std::ops::AddAssign<usize> for SymbolIndex {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs
    }
}

impl From<SymbolIndex> for usize {
    #[inline]
    fn from(v: SymbolIndex) -> usize {
        v.as_usize()
    }
}

impl From<usize> for SymbolIndex {
    #[inline]
    fn from(value: usize) -> Self {
        Self::from_usize(value)
    }
}

impl From<u32> for SymbolIndex {
    #[inline]
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl std::cmp::Eq for SymbolIndex {}

impl std::cmp::PartialEq for SymbolIndex {
    fn eq(&self, other: &Self) -> bool {
        self.as_u32().eq(&other.as_u32())
    }
}

impl std::cmp::Ord for SymbolIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u32().cmp(&other.as_u32())
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl std::cmp::PartialOrd for SymbolIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_u32().partial_cmp(&other.as_u32())
    }
}

impl std::hash::Hash for SymbolIndex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_u32().hash(state)
    }
}
