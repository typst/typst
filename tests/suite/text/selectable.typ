--- text-unselectable paged pdftags ---

// Must be tested manually!
Unselectable numbers: foo #pdf.artifact(text(selectable: false)[123 *456* #super[_789_]] + [ bar]).

--- text-unselectable-outside-artifact pdftags ---
// Error: 1:26-1:29 unselectable text must be wrapped in `pdf.artifact`
#text(selectable: false)[foo]
