// Test behaviour of the anchor element.

---

#anchor("lorem ipsum")[ #lorem(5) ] <lorem>

If in doubt, @lorem

---

#let eqn(x, caption) = stack(
    spacing: 5pt,
    x,
    anchor(caption, text(
        size: 5pt,
        align(right, [(#caption)]
    ))),
)

#eqn($ a = 42 $, [Profound Theorem#emoji.tm]) <eq1>

For more information, see @eq1

---

#anchor("correct", anchor("wrong", [Result])) <content> is @content

---

#anchor(error("fail"), [
    #anchor(error("fail"), "Hi") <ref>
    #anchor("success", "there") <ref>
    #anchor(error("fail"), "!") <ref>
]) <ref>

test: @ref

---

// Error: 15-33 can't touch this
#anchor(error("can't touch this"), [Hi]) <label>

@label

---

#anchor(error("can't touch this"), [Hi]) <label>
#anchor(error("can't touch that"), [Hi]) <label>

// Error: 1-7 label occurs multiple times in the document
@label

---

#show "Abracadabra!": anchor("magic", "No.")

\- Say the @magic words:

\- Abracadabra! <magic>

---

#let myfigure(caption, images) = anchor(
    loc => [
        #caption #counter(caption).at(loc).at(0)
    ],
    block(
        stack(
            align(horizon, grid(..images, columns: 2)),
            align(center, [
                #caption
                #counter(caption).display()

                #counter(caption).update(n => n + 1)
            ]),
        ),
        breakable: false,
    )
)

#myfigure("Shapes", (
    image("/tetrahedron.svg"),
    image("/cylinder.svg"),
)) <shapes>

#myfigure("Shapes", (
    image("/cylinder.svg"),
    image("/tetrahedron.svg"),
)) <shapes_again>

See @shapes and @shapes_again
