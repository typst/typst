// Test the spread operate in math mode

---
$#math.vec(..(1, 2, 3).map(i => $#i$))$
$vec(..#(1, 2, 3).map(i => $#i$))$

---
$#math.mat(..range(4).map(row => range(4).map(col => $u_(#row #col)$)))$
$mat(..#range(4).map(row => range(4).map(col => $u_(#row #col)$)))$
