// Test font fallback.

// Source Sans Pro + Segoe UI Emoji.
Emoji: 🏀

// CMU Serif + Noto Emoji.
[font "CMU Serif", "Noto Emoji"][
    Emoji: 🏀
]

// Class definitions.
[font serif: ("CMU Serif", "Latin Modern Math", "Noto Emoji")]
[font serif][
    Math: ∫ α + β ➗ 3
]

// Class definition reused.
[font sans-serif: "Noto Emoji"]
[font sans-serif: ("Archivo", sans-serif)]
New sans-serif. 🚀
