// Test references to footnotes.

---
A footnote #footnote[Hi]<fn> \
A reference to it @fn

---
// Multiple footnotes are refs
First #footnote[A]<fn1> \
Second #footnote[B]<fn2> \
First ref @fn1 \
Third #footnote[C] \
Fourth #footnote[D]<fn4> \
Fourth ref @fn4 \
Second ref @fn2 \
Second ref again @fn2

---
// Forward reference
Usage @fn \
Definition #footnote[Hi]<fn>

---
// Footnote ref in footnote
#footnote[Reference to next @fn]
#footnote[Reference to myself @fn]<fn>
#footnote[Reference to previous @fn]

---
// Styling
#show footnote: text.with(fill: red)
Real #footnote[...]<fn> \
Ref @fn

---
// Footnote call with label
#footnote(<fn>)
#footnote[Hi]<fn>
#ref(<fn>)
#footnote(<fn>)
