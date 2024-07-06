alias b := bundle
bundle:
  cargo run -- compile --root bundle bundle/src/main.typ out.pdf

alias s := sub
sub n:
  cargo run -- compile --root bundle bundle/src/sub{{n}}/src/main.typ out.pdf

alias pdf := see
see out="out.pdf":
  ( nohup xdg-open '{{out}}' & ) > /dev/null 2>&1

alias c := clean
clean:
  rm out.pdf
