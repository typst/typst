// Test regexes.
// Ref: false

---
{
  let re = regex("(La)?TeX")
  test(re.matches("La"), false)
  test(re.matches("TeX"), true)
  test(re.matches("LaTeX"), true)
}
