// Test var (mathematical text)

---
// Test show rule. Because the sizing is set per var,
// the equation spacing is the default (10pt for the tests)
#set text(fill:red)
#show math.var: set text(fill:blue, size:20pt)
$ a < b "iff" b > a $

---
// Test show rule.  Because a global math.var setting is made,
// spacing is at 20pt.
#set text(fill:red)
#set math.var(fill:blue, size:20pt)
$ a < b "iff" b > a $

---
// Test default italic vs. normal
$var("h") quad var("hello") quad italic(var("world"))$

---
// Test regex
#show "p": "ze"
#show sym.sum: "S"
$var("map") quad sum quad var("∑igma")$#h(1em)∑

---
// Test var/text color interactions and scoping
$x W z$
#h(12pt) $x var(size:#20pt, fill:#green, "W") z$
#h(12pt) $x var(fill:#green, "W") z$
#h(12pt) $x var(size:#20pt, "W") z$#h(4pt)
#{set math.var(fill:green); $text(#red, "time"^2)$}

---
// Test math in ordinary in math (in ordinary).
$#[whenever $x in RR$]$\
#set text(font:"New Computer Modern")
whenever $x in RR$

// ---
// Test ordinary in math vs ordinary.
//
// FIXME: There is a synthetic show rule that is pinning
// the text font for equations as New CM Math.  
// There is no such thing as italic New CM Math.
// This is a real test once the synthetic
// show rule is removed. There are kerning differences
// though, unless an ord is inserted.
$#[whenever _x_ is real]$\
#[whenever _x_ is real]

---
// Test var color applies to lines, operators 
#set math.var(fill:blue)
$cancel(sin) quad sqrt(100+z^2) quad 5/4 $

---
// Error: 2:1-2:12 current font does not support math
#set math.var(font:"Noto Sans", fallback:false)
$x^2$
---
// Test font selection, mixing.
#set math.var(font:"Fira Math")
$sin theta eq.not #{set math.var(font:"New Computer Modern Math"); $sin theta$}$
#let cmbold(x) = {
    set math.var(font:"New Computer Modern Math")
    $bold(#x)$
}
#h(1em)$bold(g) "vs" cmbold(g)$

// FIXME: test that failure happens if a var(font:"badfont") is encountered.
// FIXME: test that var(font:"newgoodfont") gets the right font; 
// this is broken now.
