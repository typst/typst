// Test locate with crazy pagebreaks.

---
#set page(height: 10pt)
{3 * locate(me => me.page * pagebreak())}
