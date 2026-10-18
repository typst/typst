// Test copy-paste and search in PDF with ligatures
// and Arabic test. Must be tested manually!

--- text-copy-paste-ligatures paged ---
The after fira 🏳️‍🌈!

#set text(lang: "ar", font: "Noto Sans Arabic")
مرحبًا

--- text-copy-paste-long-multi-byte-word pdf ---
// A word with more than 64 KiB of multi-byte text must keep valid glyph
// ranges in the PDF.
#("\u{e9}" * 32768)
