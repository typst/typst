// Test list marker configuration.

---
// Test en-dash.
#set list(marker: [--])
- A
- B

---
// Test that last item is repeated.
#set list(marker: ([--], [•]))
- A
  - B
    - C

---
// Test function.
#set list(marker: n => if n == 1 [--] else [•])
- A
- B
  - C
  - D
    - E
- F

---
// Test that bare hyphen doesn't lead to cycles and crashes.
#set list(marker: [-])
- Bare hyphen is
- a bad marker

---
// Error: 19-21 array must contain at least one marker
#set list(marker: ())
