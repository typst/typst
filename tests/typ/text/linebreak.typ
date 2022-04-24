// Test line breaks.

---
// Test overlong word that is not directly after a hard break.
This is a spaceexceedinglylongy.

---
// Test two overlong words in a row.
Supercalifragilisticexpialidocious Expialigoricmetrioxidation.

---
// Test that there are no unwanted line break opportunities on run change.
This is partly emp#emph[has]ized.

---
Hard #linebreak() break.

---
// Test hard break directly after normal break.
Hard break directly after \ normal break.

---
// Test consecutive breaks.
Two consecutive \ \ breaks and three \ \ more.

---
// Test forcing an empty trailing line.
Trailing break \ \

---
// Test justified breaks.
#set par(justify: true)
With a soft \+
break you can force a break without #linebreak(justified: true)
breaking justification. #linebreak(justified: false)
Nice!
