// Test math stretch.

--- math-stretch-basic ---
// Test basic stretch.
$ P -> Q stretch(->, size: #200%) R \
  R stretch(->) S stretch(->, size: #50%)^"epimorphism" T $

--- math-stretch-complex ---
// Test complex stretch.
$ H stretch(=)^"define" U + p V \
  x stretch(harpoons.ltrb, size: #3em) y
    stretch(\[, size: #150%) z \
  f : X stretch(arrow.hook, size: #150%)_"injective" Y \
  V stretch(->, size: #(100% + 1.5em))^("surjection") ZZ $

--- math-stretch-attach ---
// Test stretch interactions with attachments.
#set page(width: auto)

$stretch(stretch(=, size: #4em))_A$
$stretch(arrow.hook, size: #5em)^"injective map"$
$stretch(arrow.hook, size: #200%)^"injective map"$

$ P = Q
    stretch(=)^(k = 0)_(forall i) R
    stretch(=, size: #150%)^(k = 0)_(forall i) S
    stretch(=, size: #2mm)^(k = 0)_(forall i) T \
  U stretch(equiv)^(forall i)_"Chern-Weil" V
    stretch(equiv, size: #(120% + 2mm))^(forall i)_"Chern-Weil" W $

--- math-stretch-horizontal ---
// Test stretching along horizontal axis.
#let ext(sym) = math.stretch(sym, size: 2em)
$ ext(arrow.r) quad ext(arrow.l.double.bar) \
  ext(harpoon.rb) quad ext(harpoons.ltrb) \
  ext(paren.t) quad ext(shell.b) \
  ext(eq) quad ext(equiv) $

--- math-stretch-vertical ---
// Test stretching along vertical axis.
#let ext(sym) = math.stretch(sym, size: 2em)
$ ext(bar.v) quad ext(bar.v.double) quad
  ext(angle.l) quad ext(angle.r) quad
  ext(paren.l) quad ext(paren.r) \
  ext(bracket.l.double) quad ext(bracket.r.double) quad
  ext(brace.l) quad ext(brace.r) quad
  ext(bracket.l) quad ext(bracket.r) $

--- math-stretch-shorthand ---
// Test stretch when base is given with shorthand.
$stretch(||, size: #2em)$
$stretch(\(, size: #2em)$
$stretch("⟧", size: #2em)$
$stretch("|", size: #2em)$
$stretch(->, size: #2em)$
$stretch(↣, size: #2em)$

--- math-stretch-nested ---
// Test nested stretch calls.
$ stretch(=, size: #2em) \
  stretch(stretch(=, size: #4em), size: #50%) $

#let base = math.stretch($=$, size: 4em)
$ stretch(base, size: #50%) $

#let base = $stretch(=, size: #4em) $
$ stretch(base, size: #50%) $

--- math-stretch-attach-nested-equation ---
// Test stretching with attachments when nested in an equation.
#let body = $stretch(=)$
$ body^"text" $
