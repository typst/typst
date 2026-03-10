// Test shaping quirks.

--- shaping-script-separation paged ---
// Test separation by script.
#set text(font: ("Libertinus Serif", "IBM Plex Sans Devanagari"))
ABCअपार्टमेंट

// This is how it should look like.
अपार्टमेंट

// This (without the spaces) is how it would look
// if we didn't separate by script.
अ पा र् ट में ट

--- shaping-forced-script-font-feature-inhibited paged ---
// A forced `latn` script inhibits Devanagari font features.
#set text(font: ("Libertinus Serif", "IBM Plex Sans Devanagari"), script: "latn")
ABCअपार्टमेंट

--- shaping-forced-script-font-feature-enabled paged ---
// A forced `deva` script enables Devanagari font features.
#set text(font: ("Libertinus Serif", "IBM Plex Sans Devanagari"), script: "deva")
ABCअपार्टमेंट

--- issue-rtl-safe-to-break-panic paged ---
// Test that RTL safe-to-break doesn't panic even though newline
// doesn't exist in shaping output.
#set text(dir: rtl, font: "Noto Serif Hebrew")
\ ט

--- shaping-font-fallback paged ---
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
// Font fallback for emoji.
A😀B

// Font fallback for entire text.
دع النص يمطر عليك

// Font fallback in right-to-left text.
ب🐈😀سم

// Multi-layer font fallback.
Aب😀🏞سمB

// Font fallback with composed emojis and multiple fonts.
01️⃣2

// Tofus are rendered with the first font.
A🐈ዲሞB

--- shaping-emoji-basic paged ---
// This should form a three-member family.
👩‍👩‍👦

// This should form a pride flag.
🏳️‍🌈

// Skin tone modifier should be applied.
👍🏿

// This should be a 1 in a box.
1️⃣

--- shaping-emoji-bad-zwj paged ---
// These two shouldn't be affected by a zero-width joiner.
🏞‍🌋
