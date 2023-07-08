// Test code highlighting with custom syntaxes.

---
#set page(width: 180pt)
#set text(6pt)
#set raw(syntaxes: "/files/SExpressions.sublime-syntax")

```sexp
(defun factorial (x)
  (if (zerop x)
    ; with a comment
    1
    (* x (factorial (- x 1)))))
```
