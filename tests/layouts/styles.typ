{size:250pt*500pt}

_Emoji:_ Hello World! 🌍

_Styles:_ This is made *bold*, that _italic_ and this one `monospace` using the
built-in syntax!

_Styles with functions:_ This [bold][word] is made bold and [italic][that] italic
using the standard library functions `bold` and `italic`!

[italic]
Styles can also be changed through [bold] context modification.
This works basically in the same way as the built-in syntax.
_

This is not italic anymore, but still bold.
[bold]

This is completely reset. 😀

[box][
    [italic]
    Styles are scoped by boxes.
]

Outside of the box: No effect.
