# RepoPilot Learning Notes

## Slice 1 — Scan path and count files

Today I learned:
- how to create a Rust CLI project
- how to use clap for subcommands
- what PathBuf is
- how to separate CLI parsing from scanner logic
- how Result is used for filesystem errors

Architecture notes:
- main.rs handles CLI input/output
- scan/scanner.rs handles filesystem scanning
- scan/types.rs contains the result type

## Slice 2 — Recursive scanning

Today I learned:
- what recursion is
- how a function can call itself
- how to use &mut to update a shared summary
- what Default means for a struct
- what io::Result<()> means
- why scan_path is public and scan_directory is private

Architecture notes:
- scan_path is the public API of the scanner
- scan_directory is an internal recursive helper
- main.rs does not know how scanning works internally

## Slice 3 — Ignore common directories

Today I learned:
- how to define a const list in Rust
- how to read a directory name from Path
- what Option means in practice
- how let-else works
- how continue works in a loop
- how to separate traversal logic from ignore policy

Architecture notes:
- ignored directory detection lives in a helper function
- scan_directory stays focused on traversal
- ignore rules are currently hardcoded defaults

## Slice 4 — Count total lines

Today I learned:
- how to read a file with fs::read_to_string
- why reading files can return io::Result
- how to count lines with content.lines().count()
- how to use a helper function for one responsibility
- why scan_directory should not contain all file-processing logic

Architecture notes:
- scan_directory traverses the filesystem
- count_file_lines reads one file and returns line count
- ScanSummary is becoming an accumulator for scan metrics