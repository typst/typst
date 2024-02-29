// Text show-set rules are weird.

---
#show "He": set text(red)
#show "ya": set text(blue)
Heya

---
#show "Heya": set text(red)
#show   "ya": set text(blue)
Heya

---
#show "He": set text(red)
#show "Heya": set text(blue)
Heya

---
#show "Heya": set text(red)
#show   "yaho": set text(blue)
Heyaho

---
#show "He": set text(red)
#show "ya": set text(weight: "bold")
Heya

---
#show "Heya": set text(red)
#show   "ya": set text(weight: "bold")
Heya

---
#show "He": set text(red)
#show "Heya": set text(weight: "bold")
Heya

---
#show "Heya": set text(red)
#show   "yaho": set text(weight: "bold")
Heyaho
