--- module-constructor ---
#let m = module((key: "value"))
#test(m.key, "value")

--- module-dynamic-import-list ---
#let m = module((key: "value", other: "bad"))
#let other = "good"
#import m: key
#test(key,"value")
#test(other, "good")

--- module-dynamic-import-wildcard ---
#let m = module((key: "value"))
#import m: *
#test(key, "value")
