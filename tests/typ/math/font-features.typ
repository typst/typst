// Test that setting font features in math.equation has an effect.

---
$ nothing $
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $
#set math.var(features: ("cv01",), fallback: false)
$ nothing $
#show math.equation: set text(features: ("cv01",), fallback: false)
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $
