// Test symbol notation.

---
:face:
:face:unknown:
:woman:old:
:turtle:

#set text("New Computer Modern Math")
:arrow:
:arrow:l:
:arrow:r:squiggly:
#symbol(("arrow", "tr", "hook").join(":"))

---
Just a: colon. \
Still :not a symbol. \
Also not:a symbol \
:arrow:r:this and this:arrow:l: \

---
#show symbol.where(notation: "my:custom"): "MY"
This is :my:custom: notation.

---
// Error: 1-14 unknown symbol
:nonexisting:
