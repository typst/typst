// Test that gradient fills on text don't work (for now).
// Ref: false

---
// Hint: 17-43 gradients on text will be supported soon
// Error: 17-43 text fill must be a solid color
#set text(fill: gradient.linear(red, blue))
