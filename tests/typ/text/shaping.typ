// Test shaping quirks.

---
// Test separation by script.
ABCअपार्टमेंट

// This is how it should look like.
अपार्टमेंट

// This (without the spaces) is how it would look
// if we didn't separate by script.
अ पा र् ट में ट

---
// Test that RTL safe-to-break doesn't panic even though newline
// doesn't exist in shaping output.
#set text(dir: rtl, "Noto Serif Hebrew")
\ ט
