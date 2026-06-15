-/! Definition of the Iterator and IntoIterator interfaces;
-/! basically taken right from the as-named Rust trait.

pub iface Iterator is
	type Item;

	fn next &mut self -> Option Self'Item;
		fn foo bar -> None;


pub iface IntoIterator is
	type Item;
	type IntoIter: Iterator @Item=Self'Item;

	fn into_iter self -> Self'IntoIter;


-// Generic implementation of IntoIterator
-// for any type I that implements Iterator
impl IntoIterator for I where
	I: Iterator
is
	type Item is I'Item
	type IntoIter is I

	fn into_iter self -> I is
		self


fn (<$!>) lhs rhs;


fn next &mut self -> Option Self'Item where Self: Copy;
