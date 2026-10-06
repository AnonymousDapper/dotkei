

### Compiler Session
- command line options
- target config
- parser session
- IO context [session.rs:76](https://github.com/rust-lang/rust/blob/main/compiler/rustc_session/src/session.rs#L76)
- file search handler [search_paths.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_session/src/search_paths.rs)

[session.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_session/src/session.rs)

### Parser Session
- diagnostic context [rustc_errors](https://github.com/rust-lang/rust/blob/main/compiler/rustc_errors/src/lib.rs#L299)
- source map [source_map.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_span/src/source_map.rs#L185)
- symbol gallery?
- arena interner (Arena + HashTable)

[parse.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_session/src/parse.rs)


### Diagnostic Context
- emitter [emitter.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_errors/src/emitter.rs)



### Command line options
- optimizer level (defer)
- output type (defer)
- test mode (defer)
- pretty-print mode
- working dir
- color mode
