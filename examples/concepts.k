-/ Use Item ---------------------------------------------------

-/ simple use
use std;

-/ rename use
use std as stdlib;

-/ wildcard use
use std with *;

-/ nested use tree
use std with [
	self,
	io with [stdout, stdin],
	fmt with *,
	mem with Drop,
	array
];

-/ re-export nested path
pub use foo::bar::baz;


-/ Fn Item ----------------------------------------------------

-/ minimal
fn null;

-/ self-only
fn method &mut self;

-/ typed parameter and return with body
fn id x: T -> T is
	x
;

-/ constrained generic
fn format v: T -> String where T: Format is
	...
;

-/ destructured param style
fn point_x Point{x, y}: Point -> Int;


-/ Type Item --------------------------------------------------

-/ declaration only
type Foo;

-/ basic alias
type ArrayIndex = u32;

-/ generic newtype
type IOResult T is Result T IOError;

-/ constrained generic
type Map K V where K: Hash is ... ;


-/ Data (struct) Item -----------------------------------------

-/ unit struct
data Empty;

-/ tuple struct (.0 is private, .1 is public)
data Pair is (Int, pub Int);

-/ classic style
data Foo is
	bar: Int
;

-/ generic
data Id T is
	id: u32,
	__marker: Phantom T
;


-/ Enum Item --------------------------------------------------

-/ empty
enum Empty;

-/ unit variants
enum Alphabet is A, B, C;

-/ explicit discriminator
enum OpCode is
	Return = 0x01,
	Load = 0x02,
	Jump = 0x04
;

-/ generics
enum Result T E is
	Ok T,
	Err E
;

-/ complex types
enum Shape P
	where P: Coordinates
is
	Line (P, P),
	Rectangle {origin: P, bounds: P},
	Circle {origin: P, radius: Int}
;

-/ Const Item -------------------------------------------------

-/ declaration only
const KMAIN: &fn;

const label: str is "foo";


-/ Interface Item ---------------------------------------------

-/ the only associated items allowed are
-/ types, const, and functions

-/ associated functions are always public

-/ empty marker
iface Foo;

-/ basic generics
iface From T is
	fn from item: T -> Self;
;

-/ constrained
iface Hash V where V: Ord is
	...
;


-/ Implementation Item ----------------------------------------

-/ empty interface impl
impl Foo for Bar;

-/ generic interface impl
impl From Foo for Bar is
	fn from item: Foo -> Foo;
;

-/ inherent impl (combining data+behavior)
impl Foo is
	pub fn new -> Self;
;

-/ Namespacing / Access ---------------------------------------

-/ basic namespace navigation
module::submodule::Type

-/ enum variant
Enum::Variant

-/ member access
data.field

-/ method calls
object'method ...

-/ above is equivalent to
ObjectType::method object ...

-/ Expressions -------------------------------------------

-/ expr is an expression
-/ expr; is a statement

-/ match is an expression
match foo when
	pattern => ...
end

-/ loops are too
until condition loop
	...
end

-/ do-style to use a series of statements as expression
do
	...
end

-/ if expressions contain block expressions implicitly
if condition then
	...
end

-/ equivalent to
if condition then
	do
		statements
	end
end
