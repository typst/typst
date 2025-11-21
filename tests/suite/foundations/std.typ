// Test 'std', a module with the standard library

--- std-basic-access paged ---
#test(std.grid, grid)
#test(std.calc, calc)

--- std-import paged ---
#import std: grid as banana
#test(grid, banana)

--- std-of-shadowed paged ---
#let my-grid = grid[a][b]
#let grid = "oh no!"
#test(my-grid.func(), std.grid)

--- std-shadowing paged ---
#let std = 5
// Error: 6-10 cannot access fields on type integer
#std.grid

--- std-mutation paged ---
// Error: 3-6 cannot mutate a constant: std
#(std = 10)

--- std-shadowed-mutation paged ---
#let std = 10
#(std = 7)
#test(std, 7)

--- std-math paged ---
$ std.rect(x + y = 5) $
