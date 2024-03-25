// Big numbers (larger than what i64 can store) should just lose some precision
// but not overflow
// https://github.com/typst/typst/issues/3363
// Ref: false

#let bignum = json("/assets/data/big-number.json")

#bignum