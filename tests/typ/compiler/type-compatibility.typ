// Test compatibility between types and strings.
// Ref: false

---
#test(type(10), int)
#test(type(10), "integer")
#test("is " + type(10), "is integer")
#test(int in ("integer", "string"), true)
#test(int in "integers or strings", true)
#test(str in "integers or strings", true)
