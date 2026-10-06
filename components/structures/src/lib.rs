// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

pub use hashbrown::hash_table;

pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};

pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, FxBuildHasher>;

pub type IndexEntry<'a, K, V> = indexmap::map::Entry<'a, K, V>;
