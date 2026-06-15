-/! module level doc
-/! at some point i may need to use different syntax
-/! for these "inner" items, but w/e

-// Span {start, end}
-// this doc comment applies to this
-// entire declaration
pub data Span is
	-/! this doc comment also applies
	-/! to the data declaration

	-// this one applies to `start`
	pub start: usize,

	pub end: usize


-// Token {kind, span}
data Token is
	pub kind: TokenKind,
	pub span: Span

-// TokenKind
pub enum TokenKind is
	Ident,

	Keyword KWType,

	String CopyOnWrite String,

	Error {type: ErrorType, skip: bool},

	RandomBool (bool, String)



-/ line comment

-// doc comment (inner)

-/! doc comment (outer)
