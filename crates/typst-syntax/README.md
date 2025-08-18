# typst-syntax

Welcome to the Typst Syntax crate! This crate manages the syntactical structure
of Typst by holding some core abstractions like assigning source file ids,
parsing Typst syntax, creating an Abstract Syntax Tree (AST), initializing
source "spans" (for linking AST elements to their outputs in a document), and
syntax highlighting.

Below are quick descriptions of the files you might be editing if you find
yourself here :)

- `lexer.rs`: The lexical foundation of the parser, which converts a string of
  characters into tokens.
- `parser.rs`: The main parser definition, preparing a Concrete Syntax Tree made
  of nested vectors of `SyntaxNode`s.
- `reparser.rs`: The algorithm for reparsing the minimal required amount of
  source text for efficient incremental compilation.
- `ast.rs`: The conversion layer between the Concrete Syntax Tree of the parser
  and the Abstract Syntax Tree used for code evaluation.
- `node.rs` & `span.rs`: The underlying data structure for the Concrete Syntax
  Tree and the definitions of source spans used for efficiently pointing to a
  syntax node in things like diagnostics.
- `kind.rs` & `set.rs`: An enum with all syntactical tokens and nodes and
  bit-set data structure for sets of `SyntaxKind`s.
- `highlight.rs`: Extracting of syntax highlighting information out of the
  Concrete Syntax Tree (and outputting as HTML).
- `path.rs`, `file.rs`, `package.rs`: The system for interning project and
  package paths as unique file IDs and resolving them in a virtual filesystem
  (not actually for _opening_ files).

The structure of the parser is largely adapted from Rust Analyzer. Their
[documentation][ra] is a good reference for a number of the design decisions
around the parser and AST.

The reparsing algorithm is explained in Section 4 of [Martin's thesis][thesis]
(though it changed a bit since).

[ra]: https://github.com/rust-lang/rust-analyzer/blob/master/docs/book/src/contributing/syntax.md
[thesis]:
    https://www.researchgate.net/publication/364622490_Fast_Typesetting_with_Incremental_Compilation
