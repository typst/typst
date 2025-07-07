// Test shorthands for unicode codepoints.

--- shorthand-nbsp-and-shy-hyphen ---
The non-breaking space~does work, soft-?hyphen does, and non-!breaking hyphen also does.

--- shorthand-nbsp-width ---
// Make sure non-breaking and normal space always
// have the same width. Even if the font decided
// differently.
#set text(font: "New Computer Modern")
a b \
a~b

--- shorthand-dashes ---
- En dash: --
- Em dash: ---

--- shorthand-ellipsis ---
#set text(font: "Roboto")
A... vs #"A..."

--- shorthand-minus ---
// Make sure shorthand is applied only before a digit.
-a -1

--- shorthands-math ---
// Check all math shorthands.
$...$\
$-$\
$'$\
$*$\
$~$\
$!=$\
$:=$\
$::=$\
$=:$\
$<<$\
$<<<$\
$>>$\
$>>>$\
$<=$\
$>=$\
$->$\
$-->$\
$|->$\
$>->$\
$->>$\
$<-$\
$<--$\
$<-<$\
$<<-$\
$<->$\
$<-->$\
$~>$\
$~~>$\
$<~$\
$<~~$\
$=>$\
$|=>$\
$==>$\
$<==$\
$<=>$\
$<==>$\
$[|$\
$|]$\
$||$
