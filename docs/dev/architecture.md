# Typst Compiler Architecture
Wondering how to contribute or just curious how Typst works? This document
covers the general structure and architecture of Typst's compiler, so you get an
understanding of what's where and how everything fits together.


## Directories
Let's start with a broad overview of the directories in this repository:

- `crates/typst`: The main compiler crate which defines the complete language
  and library.
- `crates/typst-cli`: Typst's command line interface. This is a relatively small
  layer on top of the compiler and the exporters.
- `crates/typst-eval`: The interpreter for the Typst language.
- `crates/typst-html`: The HTML exporter.
- `crates/typst-ide`: Exposes IDE functionality.
- `crates/typst-kit`: Contains various default implementation of
  functionality used in `typst-cli`.
- `crates/typst-layout`: Typst's layout engine.
- `crates/typst-library`: Typst's standard library.
- `crates/typst-macros`: Procedural macros for the compiler.
- `crates/typst-pdf`: The PDF exporter.
- `crates/typst-realize`: Typst's realization subsystem.
- `crates/typst-render`: A renderer for Typst frames.
- `crates/typst-svg`: The SVG exporter.
- `crates/typst-syntax`: Home to the parser and syntax tree definition.
- `crates/typst-timing`: Performance timing for Typst.
- `crates/typst-utils`: Utilities for Typst.
- `docs`: Generates the content of the official
  [documentation][docs] from markdown files and the inline
  Rust documentation. Only generates the content and structure, not the concrete
  HTML (that part is currently closed source).
- `tests`: Integration tests for Typst compilation.
- `tools`: Tooling for development.


## Compilation
The source-to-PDF compilation process of a Typst file proceeds in four phases.

1. **Parsing:** Turns a source string into a syntax tree.
2. **Evaluation:** Turns a syntax tree and its dependencies into content.
3. **Layout:** Layouts content into frames.
4. **Export:** Turns frames into an output format like PDF or a raster graphic.

The Typst compiler is _incremental:_ Recompiling a document that was compiled
previously is much faster than compiling from scratch. Most of the hard work is
done by [`comemo`], an incremental compilation framework we have written for
Typst. However, the compiler is still carefully written with incrementality in
mind. Below we discuss the four phases and how incrementality affects each of
them.


## Parsing
The syntax tree and parser are located in `crates/typst-syntax`. Parsing is
a pure function `&str -> SyntaxNode` without any further dependencies. The
result is a concrete syntax tree reflecting the whole file structure, including
whitespace and comments. Parsing cannot fail. If there are syntactic errors, the
returned syntax tree contains error nodes instead. It's important that the
parser deals well with broken code because it is also used for syntax
highlighting and IDE functionality.

**Typedness:**
The syntax tree is untyped, any node can have any `SyntaxKind`. This makes it
very easy to (a) attach spans to each node (see below), (b) traverse the tree
when doing highlighting or IDE analyses (no extra complications like a visitor
pattern). The `typst::syntax::ast` module provides a typed API on top of
the raw tree. This API resembles a more classical AST and is used by the
interpreter.

**Spans:**
After parsing, the syntax tree is numbered with _span numbers._ These numbers
are unique identifiers for syntax nodes that are used to trace back errors in
later compilation phases to a piece of syntax. The span numbers are ordered so
that the node corresponding to a number can be found quickly.

**Incremental:**
Typst has an incremental parser that can reparse a segment of markup or a
code/content block. After incremental parsing, span numbers are reassigned
locally. This way, span numbers further away from an edit stay mostly stable.
This is important because they are used pervasively throughout the compiler,
also as input to memoized functions. The less they change, the better for
incremental compilation.


## Evaluation
The evaluation phase lives in `crates/typst-eval`. It takes a parsed
`Source` file and evaluates it to a `Module`. A module consists of the `Content`
that was written in it and a `Scope` with the bindings that were defined within
it.

A source file may depend on other files (imported sources, images, data files),
which need to be resolved. Since Typst is deployed in different environments
(CLI, web app, etc.) these system dependencies are resolved through a general
interface called a `World`. Apart from files, the world also provides
configuration and fonts.

**Interpreter:**
Typst implements a tree-walking interpreter. To evaluate a piece of source, you
first create a `Vm` with a scope stack. Then, the AST is recursively evaluated
through trait impls of the form `fn eval(&self, vm: &mut Vm) -> Result<Value>`.
An interesting detail is how closures are dealt with: When the interpreter sees
a closure / function definition, it walks the body of the closure and finds all
accesses to variables that aren't defined within the closure. It then clones the
values of all these variables (it _captures_ them) and stores them alongside the
closure's syntactical definition in a closure value. When the closure is called,
a fresh `Vm` is created and its scope stack is initialized with the captured
variables.

**Incremental:**
In this phase, incremental compilation happens at the granularity of the module
and the closure. Typst memoizes the result of evaluating a source file across
compilations. Furthermore, it memoizes the result of calling a closure with a
certain set of parameters. This is possible because Typst ensures that all
functions are pure. The result of a closure call can be recycled if the closure
has the same syntax and captures, even if the closure values stems from a
different module evaluation (i.e. if a module is reevaluated, previous calls to
closures defined in the module can still be reused).


## Layout
The layout phase takes `Content` and produces one `Frame` per page for it. To
layout `Content`, we first have to _realize_ it by applying all relevant show
rules to the content. Since show rules may be defined as Typst closures,
realization can trigger closure evaluation, which in turn produces content that
is recursively realized. Realization is a shallow process: While collecting list
items into a list that we want to layout, we don't realize the content within
the list items just yet. This only happens lazily once the list items are
layouted.

When we a have realized the content into a layoutable element, we can then
layout it into _regions,_ which describe the space into which the content shall
be layouted. Within these, an element is free to layout itself as it sees fit,
returning one `Frame` per region it wants to occupy.

**Introspection:**
How content layouts (and realizes) may depend on how _it itself_ is layouted
(e.g., through page numbers in the table of contents, counters, state, etc.).
Typst resolves these inherently cyclical dependencies through the _introspection
loop:_ The layout phase runs in a loop until the results stabilize. Most
introspections stabilize after one or two iterations. However, some may never
stabilize, so we give up after five attempts.

**Incremental:**
Layout caching happens at the granularity of the element. This is important
because overall layout is the most expensive compilation phase, so we want to
reuse as much as possible.


## Export
Exporters live in separate crates. They turn layouted frames into an output file
format.

- The PDF exporter takes layouted frames and turns them into a PDF file.
- The SVG exporter takes a frame and turns it into an SVG.
- The built-in renderer takes a frame and turns it into a pixel buffer.
- HTML export does not exist yet, but will in the future. However, this requires
  some complex compiler work because the export will start with `Content`
  instead of `Frames` (layout is the browser's job).


## IDE
The `crates/typst-ide` crate implements IDE functionality for Typst. It
builds heavily on the other modules (most importantly, `syntax` and `eval`).

**Syntactic:**
Basic IDE functionality is based on a file's syntax. However, the standard
syntax node is a bit too limited for writing IDE tooling. It doesn't provide
access to its parents or neighbours. This is fine for an evaluation-like
recursive traversal, but impractical for IDE use cases. For this reason, there
is an additional abstraction on top of a syntax node called a `LinkedNode`,
which is used pervasively across the `ide` module.

**Semantic:**
More advanced functionality like autocompletion requires semantic analysis of
the source. To gain semantic information for things like hover tooltips, we
directly use other parts of the compiler. For instance, to find out the type of
a variable, we evaluate and realize the full document equipped with a `Tracer`
that emits the variable's value whenever it is visited. From the set of
resulting values, we can then compute the set of types a value takes on. Thanks
to incremental compilation, we can recycle large parts of the compilation that
we had to do anyway to typeset the document.

**Incremental:**
Syntactic IDE stuff is relatively cheap for now, so there are no special
incrementality concerns. Semantic analysis with a tracer is relatively
expensive. However, large parts of a traced analysis compilation can reuse
memoized results from a previous normal compilation. Only the module evaluation
of the active file and layout code that somewhere within evaluates source code
in the active file needs to re-run. This is all handled automatically by
`comemo` because the tracer is wrapped in a `comemo::TrackedMut` container.


## Tests
Typst has an extensive suite of integration tests. These tests cover parsing,
evaluation, realization, layout, and rendering. PDF output is sadly untested so
far, but most bugs are in earlier phases of the compiler. For more details about
testing, see the [tests directory](/tests) and its README.

[docs]: https://typst.app/docs/
[`comemo`]: https://github.com/typst/comemo/
