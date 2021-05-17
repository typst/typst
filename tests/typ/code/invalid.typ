// Test invalid control syntax.

---
// Error: 5 expected identifier
#let

// Error: 5 expected identifier
{let}

// Error: 6-9 expected identifier, found string
#let "v"

// Should output `1`.
// Error: 7 expected semicolon or line break
#let v 1

// Error: 9 expected expression
#let v =

// Should output `= 1`.
// Error: 6-9 expected identifier, found string
#let "v" = 1

---
// Error: 4 expected expression
#if

// Error: 4 expected expression
{if}

// Error: 6 expected body
#if x

// Error: 1-6 unexpected keyword `else`
#else {}

// Should output `x`.
// Error: 4 expected expression
#if
x {}

// Should output `something`.
// Error: 6 expected body
#if x something

// Should output `A thing.`
// Error: 20 expected body
A#if false {} #else thing

---
// Error: 7 expected expression
#while

// Error: 7 expected expression
{while}

// Error: 9 expected body
#while x

// Should output `x`.
// Error: 7 expected expression
#while
x {}

// Should output `something`.
// Error: 9 expected body
#while x something

---
// Error: 5 expected identifier
#for

// Error: 5 expected identifier
{for}

// Error: 7 expected keyword `in`
#for v

// Error: 10 expected expression
#for v in

// Error: 15 expected body
#for v in iter

// Should output `v in iter`.
// Error: 5 expected identifier
#for
v in iter {}

// Should output `A thing`.
// Error: 7-10 expected identifier, found string
A#for "v" thing

// Should output `in iter`.
// Error: 6-9 expected identifier, found string
#for "v" in iter {}

// Should output `+ b in iter`.
// Error: 7 expected keyword `in`
#for a + b in iter {}
