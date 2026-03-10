// Test shorthands for unicode codepoints.

--- shorthand-nbsp-and-shy-hyphen paged ---
The non-breaking space~does work, soft-?hyphen also does.

--- shorthand-nbsp-width paged ---
// Make sure non-breaking and normal space always
// have the same width. Even if the font decided
// differently.
#set text(font: "New Computer Modern")
a b \
a~b

--- shorthand-dashes paged ---
- En dash: --
- Em dash: ---

--- shorthand-ellipsis paged ---
#set text(font: "Roboto")
A... vs #"A..."

--- shorthand-minus paged ---
// Make sure shorthand is applied only before a digit.
-a -1

--- shorthands-math paged ---
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
