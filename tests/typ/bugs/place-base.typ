// Test that placement is relative to container and not itself.

---
#set page(height: 80pt, margin: 0pt)
#place(right, dx: -70%, dy: 20%, [First])
#place(left, dx: 20%, dy: 60%, [Second])
#place(center + horizon, dx: 25%, dy: 25%, [Third])
