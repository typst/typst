// Test font fallback.

// Source Sans Pro + Segoe UI Emoji.
Emoji: 🏀

// CMU Serif + Noto Emoji.
[font "CMU Serif", "Noto Emoji"][Emoji: 🏀]

// Class definitions.
[font math: ("CMU Serif", "Latin Modern Math", "Noto Emoji")]
[font math][Math: ∫ α + β ➗ 3]

// Class redefinition.
[font sans-serif: "Noto Emoji"]
[font sans-serif: ("Archivo", sans-serif)]
New sans-serif. 🚀

// TODO: Add tests for other scripts.
