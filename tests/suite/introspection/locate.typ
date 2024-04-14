--- locate-position ---
// Test `locate`.
#v(10pt)
= Introduction <intro>
#context test(locate(<intro>).position().y, 20pt)

--- locate-missing-label ---
// Error: 10-25 label `<intro>` does not exist in the document
#context locate(<intro>)

--- locate-duplicate-label ---
= Introduction <intro>
= Introduction <intro>

// Error: 10-25 label `<intro>` occurs multiple times in the document
#context locate(<intro>)

--- locate-element-selector ---
#v(10pt)
= Introduction <intro>
#context test(locate(heading).position().y, 20pt)

--- locate-element-selector-no-match ---
// Error: 10-25 selector does not match any element
#context locate(heading)

--- locate-element-selector-multiple-matches ---
= Introduction <intro>
= Introduction <intro>

// Error: 10-25 selector matches multiple elements
#context locate(heading)
