// Test shorthands for unicode codepoints.

--- shorthand-nbsp-and-shy-hyphen render ---
The non-breaking space~does work, soft-?hyphen also does.

--- shorthand-nbsp-width render ---
// Make sure non-breaking and normal space always
// have the same width. Even if the font decided
// differently.
#set text(font: "New Computer Modern")
a b \
a~b

--- shorthand-dashes render ---
- En dash: --
- Em dash: ---

--- shorthand-ellipsis render ---
#set text(font: "Roboto")
A... vs #"A..."

--- shorthand-minus render ---
// Make sure shorthand is applied only before a digit.
-a -1

--- shorthands-math render ---
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
