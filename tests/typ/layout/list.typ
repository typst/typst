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
// Apply listitem styles to markers as well.
- This is a regular item and has a regular marker
#text(blue)[- This has a blue marker]
- This is a regular item and has a regular marker
#hide[- This is a hidden item, its marker is also hidden]
- The above was a hidden item (with a hidden marker)
#strong[- Bold item and marker]

#text(red, list[This applies red to the entire list])

---
// Do not apply styles to markers when they only apply to the item body.
- Regular item and marker
- #text(blue)[Blue item body, but regular marker]
- #hide[Hidden item body, but regular marker]
- #strong[Bold item, but regular marker]
#text(red)[- #text(blue)[The item body is blue, but the marker is red]]
#list(text(blue)[Marker should also be regular here, despite the blue text])
