
# typst-syntax

Welcome to the Typst Syntax crate! This crate manages the syntactical
structure of Typst by holding some core abstractions like assigning
source file ids, parsing Typst syntax, creating an Abstract Syntax Tree
(AST), initializing source 'spans' (for linking AST elements to their
outputs in a document), and basic syntax highlighting.

The structure of the parser is largely adapted from Rust Analyzer. Their
documentation is a good reference for a number of the design decisions
around the parser and AST:
[see here](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/syntax.md)

The reparsing algorithm is explained in section 4 of Martin's thesis:
[see here](https://scholar.google.com/scholar?hl=en&as_sdt=0%2C44&q=Fast+Typesetting+with+Incremental+Compilation&btnG=)

Below are quick descriptions of the files you might be editing if you
find yourself here :)

- `parser.rs` & `set.rs`: The main parser definition, preparing a
  Concrete Syntax Tree of nested vectors of SyntaxNodes.
- `ast.rs`: The conversion layer between the Concrete Syntax Tree of the
  parser and the Abstract Syntax Tree used for code evaluation.
- `node.rs` & `span.rs`: The underlying data structure for the Concrete
  Syntax Tree and the definitions of source Spans.
- `highlight.rs`: Interpreting the Concrete Syntax Tree into
  highlighting information (and outputting as HTML).
- `lexer.rs` & `syntax.rs`: The lexical foundation of the parser,
  together these define the individual tokens available for the parser
  and their valid representations.
- `path.rs`, `file.rs`, `package.rs`: The system for interning package
  paths as unique file IDs and resolving them in the filesystem (not
  actually for *opening* files).
- `reparser.rs`: The algorithm for reparsing the minimal required amount
  of source text for efficient incremental 
