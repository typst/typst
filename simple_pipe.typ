#let test(x,y) = if x != y {panic()}

#let pos(..x) = x.pos() 
#{  
test((2,4).. |> pos(1,_,3,_), pos(1,2,3,4))
}  

