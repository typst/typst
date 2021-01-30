// Ends paragraphs.
Tightly #[v -5pt] packed

// Eating up soft spacing.
Inv #[h 0pt] isible

// Multiple spacings in a row.
Add #[h 10pt] #[h 10pt] up

// Relative to font size.
Relative #[h 100%] spacing

// Missing spacing.
// Error: 12-12 missing argument: spacing
Totally #[h] ignored

// Swapped axes.
#[page main-dir: rtl, cross-dir: ttb, height: 80pt][
    1 #[h 1cm] 2

    3 #[v 1cm] 4 #[v -1cm] 5
]
