// Test complex text shaping.

---
// Test ligatures.

// This should create an "fi" ligature.
Le fira

// This should just shape nicely.
#font("Noto Sans Arabic")
منش إلا بسم الله

// This should form a three-member family.
#font("Twitter Color Emoji")
👩‍👩‍👦 🤚🏿

// These two shouldn't be affected by a zero-width joiner.
🏞‍🌋

---
// Test font fallback.

#font("EB Garamond", "Noto Sans Arabic", "Twitter Color Emoji")

// Font fallback for emoji.
A😀B

// Font fallback for entire text.
منش إلا بسم الله

// Font fallback in right-to-left text.
ب🐈😀سم

// Multi-layer font fallback.
Aب😀🏞سمB

// Tofus are rendered with the first font.
A🐈中文B
