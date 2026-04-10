# Math Sub-numbering Testing Guide

This document describes how to run tests for the math sub-numbering feature in Typst.

## Overview

The sub-numbering feature allows multiline equations to have individual line numbers (e.g., `(1a)`, `(1b)`) and supports referencing specific lines with labels.

## Test File Location

The main test file for sub-numbering is located at:

```bash
tests/suite/math/sub-numbering.typ
```

## Running Tests

### Run all tests

```bash
cargo test --package typst-tests
```

### Run only sub-numbering tests

```bash
cargo test --package typst-tests -- sub-numbering
```

### Update test references

When you make changes that affect the visual output, you need to update the reference files:

```bash
cargo test --package typst-tests -- --update sub-numbering

# remove dangle resources
cargo test --package typst-tests -- undangle
```

### Run all math tests

```bash
cargo test --package typst-tests -- math
```

## Test Cases

The `sub-numbering.typ` test file includes the following test cases:

| Test Case                               | Description                                   |
| --------------------------------------- | --------------------------------------------- |
| `math-sub-numbering-basic`              | Basic sub-numbering with default settings     |
| `math-sub-numbering-disabled`           | Sub-numbering disabled globally               |
| `math-sub-numbering-single-line`        | Single-line equations unaffected              |
| `math-sub-numbering-manual-enable`      | Manually enable numbering for specific lines  |
| `math-sub-numbering-manual-disable`     | Manually disable numbering for specific lines |
| `math-sub-numbering-alignment`          | Sub-number alignment options                  |
| `math-sub-numbering-with-pagebreak`     | Page breaking with sub-numbering              |
| `math-sub-numbering-pattern`            | Different sub-numbering patterns              |
| `math-sub-numbering-multiple-equations` | Multiple equations in sequence                |
| `math-sub-numbering-empty-lines`        | Empty lines in equations                      |
| `math-sub-numbering-reference`          | Referencing sub-equations with labels         |

## Test Syntax

### Basic sub-numbering

```typ
#set math.equation(numbering: "(1)", sub-number: true)

$ E &= m c^2 \
     &= p c + ... $
```

### Manual control

```typ
#set math.equation(numbering: "(1)", sub-numbering: false)

$ E &= m c^2 & #[#math.line(numbering: true)] \
     &= p c + ... $
```

### Referencing sub-equations

```typ
#set math.equation(numbering: "(1)", sub-numbering: true)

$ E &= m c^2 & #[#math.line() <einstein>] \
     &= p c + ... & #[#math.line() <approx>] $

See @einstein for the energy-mass relation.
```

## Reference Output Files

Test reference outputs are stored in:

- `tests/ref/render/` - PNG images of rendered output
- `tests/ref/svg/hashes.txt` - SVG output hashes
- `tests/ref/pdf/hashes.txt` - PDF output hashes

## Adding New Tests

To add a new test case:

1. Add a new test section to `tests/suite/math/sub-numbering.typ`:

```typ
--- your-test-name paged ---
// Your test code here
```

2. Run the test with `--update` to generate reference outputs:

```bash
cargo test --package typst-tests -- --update your-test-name
```

3. Verify the generated reference output looks correct.

## Related Files

- Implementation: `crates/typst-library/src/math/equation.rs`
- Layout: `crates/typst-layout/src/math/mod.rs`
- IR processing: `crates/typst-library/src/math/ir/` (item.rs, process.rs, resolve.rs)
