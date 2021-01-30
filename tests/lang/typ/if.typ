#let x = true

// The two different bodies.
#if true [_1_,] #if x {"2"}

// Braced condition is fine.
#if {true} {"3"}

// Newlines.
#if false [

] #else [
    4
]

// Multiline (condition needs parens because it's terminated by the line break,
// just like the right-hand side of a let-binding).
#if (
    x
) {
    "Fi" + "ve"
}

// Spacing is somewhat delicate. We only want to have spacing in the output if
// there was whitespace before/after the full if-else statement. In particular,
// spacing after a simple if should be retained, but spacing between the first
// body and the else should be ignored.
a#if true[b]c \
a#if true[b] c \
a #if true{"b"}c \
a #if true{"b"} c \
a#if false [?] #else [b]c \
a#if true {"b"} #else {"?"} c \

// Body not evaluated at all if condition is false.
#if false { dont-care-about-undefined-variables }

---
#let x = true

// Needs condition.
// Error: 1:6-1:7 expected expression, found closing brace
a#if }

// Needs if-body.
// Error: 2:7-2:7 expected body
// Error: 1:16-1:16 expected body
a#if x b#if (x)c

// Needs else-body.
// Error: 1:20-1:20 expected body
a#if true [b] #else c

// Lone else.
// Error: 1:1-1:6 unexpected keyword `#else`
#else []

// Condition must be boolean. If it isn't, neither branch is evaluated.
// Error: 1:5-1:14 expected boolean, found string
#if "a" + "b" { "nope" } #else { "nope" }

// No coercing from empty array or or stuff like that.
// Error: 1:5-1:7 expected boolean, found array
#if () { "nope" } #else { "nope" }
