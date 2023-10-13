// Test that setting font features in math.equation has an effect.

---
$ nothing $
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $
#show math.equation: set text(features: ("cv01",), fallback: false)
$ nothing $
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $
