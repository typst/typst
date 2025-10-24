// Test 'std', a module with the standard library

--- std-basic-access render ---
#test(std.grid, grid)
#test(std.calc, calc)

--- std-import render ---
#import std: grid as banana
#test(grid, banana)

--- std-of-shadowed render ---
#let my-grid = grid[a][b]
#let grid = "oh no!"
#test(my-grid.func(), std.grid)

--- std-shadowing render ---
#let std = 5
// Error: 6-10 cannot access fields on type integer
#std.grid

--- std-mutation render ---
// Error: 3-6 cannot mutate a constant: std
#(std = 10)

--- std-shadowed-mutation render ---
#let std = 10
#(std = 7)
#test(std, 7)

--- std-math render ---
$ std.rect(x + y = 5) $
