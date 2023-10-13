/// Test markup lints.
// Ref: false

---
// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
**

---
// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
// Warning: 11-13 no text within stars
// Hint: 11-13 using multiple consecutive stars (e.g. **) has no additional effect
**not bold**

---
// Warning: 1-3 no text within underscores
// Hint: 1-3 using multiple consecutive underscores (e.g. __) has no additional effect
__

---
// Warning: 1-3 no text within underscores
// Hint: 1-3 using multiple consecutive underscores (e.g. __) has no additional effect
// Warning: 13-15 no text within underscores
// Hint: 13-15 using multiple consecutive underscores (e.g. __) has no additional effect
__not italic__
