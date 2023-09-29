// Test that the color of a raw block is not overwritten
// this is done by checking two things: luma == rgb and
// that the color shown is correct

---

#show raw: set text(fill: blue)

`Hello, World!`

```rs
fn main() {
    println!("Hello, World!");
}
```

---
// Ref: false

#test(rgb(17%, 17%, 17%), luma(17%))
#test(luma(17%), rgb(17%, 17%, 17%))
#test(rgb(99%, 99%, 99%), luma(99%))
#test(luma(99%), rgb(99%, 99%, 99%))
