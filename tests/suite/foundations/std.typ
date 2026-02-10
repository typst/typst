// Test 'std', a module with the standard library

--- std-basic-access eval ---
#test(std.grid, grid)
#test(std.calc, calc)

--- std-import eval ---
#import std: grid as banana
#test(grid, banana)

--- std-of-shadowed eval ---
#let my-grid = grid[a][b]
#let grid = "oh no!"
#test(my-grid.func(), std.grid)

--- std-shadowing eval ---
#let std = 5
// Error: 6-10 cannot access fields on type integer
#std.grid

--- std-mutation eval ---
// Error: 3-6 cannot mutate a constant: std
#(std = 10)

--- std-shadowed-mutation eval ---
#let std = 10
#(std = 7)
#test(std, 7)

--- std-math paged ---
$ std.rect(x + y = 5) $
