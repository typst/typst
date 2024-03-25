// Big numbers (larger than what i64 can store) should be saturating
// and not overflowing
// https://github.com/typst/typst/issues/3363

#let bignum = json("/assets/data/big-number.json")

#bignum