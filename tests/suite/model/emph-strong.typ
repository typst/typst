// Test emph and strong.

--- emph-syntax paged ---
// Basic.
_Emphasized and *strong* words!_

// Inside of a word it's a normal underscore or star.
hello_world Nutzer*innen

// CJK characters will not need spaces.
中文一般使用*粗体*或者_楷体_来表示强调。

日本語では、*太字*や_斜体_を使って強調します。

中文中混有*Strong*和_Emphasis_。

// Can contain paragraph in nested content block.
_Still #[

] emphasized._

--- emph-and-strong-call-in-word paged ---
// Inside of words can still use the functions.
P#strong[art]ly em#emph[phas]ized.

--- emph-empty-hint paged ---
// Warning: 1-3 no text within underscores
// Hint: 1-3 using multiple consecutive underscores (e.g. __) has no additional effect
__

--- emph-double-underscore-empty-hint paged ---
// Warning: 1-3 no text within underscores
// Hint: 1-3 using multiple consecutive underscores (e.g. __) has no additional effect
// Warning: 13-15 no text within underscores
// Hint: 13-15 using multiple consecutive underscores (e.g. __) has no additional effect
__not italic__

--- emph-unclosed paged ---
// Error: 6-7 unclosed delimiter
#box[_Scoped] to body.

--- emph-ends-at-parbreak paged ---
// Ends at paragraph break.
// Error: 1-2 unclosed delimiter
_Hello

World

--- emph-strong-unclosed-nested paged ---
// Error: 11-12 unclosed delimiter
// Error: 3-4 unclosed delimiter
#[_Cannot *be interleaved]

--- strong-delta paged ---
// Adjusting the delta that strong applies on the weight.
Normal

#set strong(delta: 300)
*Bold*

#set strong(delta: 150)
*Medium* and *#[*Bold*]*

--- strong-empty-hint paged ---
// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
**

--- strong-double-star-empty-hint paged ---
// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
// Warning: 11-13 no text within stars
// Hint: 11-13 using multiple consecutive stars (e.g. **) has no additional effect
**not bold**
