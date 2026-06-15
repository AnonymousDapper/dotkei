-/! module level doc comment

-/ module level comment

-// statement level doc comment
use std::io;

use core::hal::abstract::a with *;

use std with [
	self,
	io with [stdout, stdin],
	fmt with *,
	mem with Drop,
	array as ar
];
