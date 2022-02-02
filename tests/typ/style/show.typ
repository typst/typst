// Test show rules.

---
// Error: 1-34 show rules are not yet implemented
#show heading(body) as [*{body}*]

---
// Error: 2-15 show is only allowed directly in markup
{show (a) as b}
