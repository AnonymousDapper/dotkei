# Dotkei Planning

## Frontend

### Lexer
- byte scanning
- individual token scanning

### Parser
- yes

### Source Manager
- multiple file support
- repl source (continue signaling)

### Error Reporting
- ariadne (for now)

See [examples/concepts.k](../examples/concepts.k)

TODO: SourceMap, diagctx, Span


```
SourceMap {
	files: SourceFileCache
	file_loader: FileLoader
	working_dir: FileName
}

SourceFileCache {
	source_files: Vec<SourceFile>
	id_cache: Map<FileId, SourceFile>
}

FileLoader {
	fn file_exists
	fn read_file
	fn cwd
}

SourceFile {
	name: FileName
	src: String
	start_pos: BytePos
	len: u32
	lines: SourceFileLines
	stable_id: FileId
}

FileName {
	name: Path
	working_dir: Path
}

BytePos (u32)

RelativeBytePos (u32)

SourceFileLines (Vec<RelativeBytePos>)

FileId (StableHash)



```
