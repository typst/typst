// Test bullet lists.

---
_Shopping list_
#list[Apples][Potatoes][Juice]

---
- First level.

  - Second level.
    There are multiple paragraphs.

    - Third level.

    Still the same bullet point.

  - Still level 2.

- At the top.

---
- Level 1
  - Level #[
2 through content block
]

---
  - Top-level indent
- is fine.

---
 - A
     - B
   - C
- D

---
// This works because tabs are used consistently.
	- A with 1 tab
		- B with 2 tabs

---
// This doesn't work because of mixed tabs and spaces.
  - A with 2 spaces
		- B with 2 tabs

---
// Edge cases.
-
Not in list
-Nope

---
// Alignment shouldn't affect marker
#set align(horizon)

- ABCDEF\ GHIJKL\ MNOPQR
