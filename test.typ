=== Brakets

$⟨x | 1/(1/2)⟩$

$lr(angle.l x | 1 / (1/2)  z angle.r)$

$⟨x | 1/(1/2) | y⟩$

$⟨x | 1/(1/2)abs(a) | y⟩$

parser interferes: $⟨x | 1/(1/2)|a| | y⟩$

parser interferes: $⟨x | 1/(1/2)a | b | y⟩$

$braket(x | 1/(1/2) | y)$

todo, replace <>?: $lr(< x | 1/(1/2) | y >)$

=== Sets

${ x | x < 1 / (1/2) }$

${ x | |x| < 1 / (1/2) }$

must use abs (though no better way to do this): ${ x | abs(1/(1/x)) < 3 }$

escape scaling: $lr(scale: "delim", \{ x | x < 1 / (1/2) \})$

$set(x | x < 1 / (1/2))$

=== `mid`

${ x mid(|) x < 1 / (1/2) }$

${ x mid(()) x < 1 / (1/2) }$

=== Ketbra (projection operators)

$|1/(1/2)><x|$

not interpreted as bracketed $|1/(1/2)⟩⟨x|$ <= `$|..⟩⟨x|$`

doesn't compile `$lr(|x⟩⟨x|)$`

```
error: expected closing paren
   ┌─ test.typ:39:11
   │
39 │ $lr(|x⟩⟨x|)$
   │            ^
```

$lr(|x><1/(1/2)|)$

$lr(|x><1/(1/2)|)$

spacing breaks it: $ketbra(x angle.r angle.l 1/(1/planck.reduce))$ <= `$ketbra(x angle.r angle.l ..)$`

$ketbra(x><y 1/(1/planck.reduce))$

unscaled: $ket(x)bra(y 1/(1/2))$

=== Probability
$P(A | 1 / (1/2) )$
