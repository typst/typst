// Test suppression of warnings in transient files.
// Ref: false
---
#import "@test/warner:0.1.0": cause_warn

// Warning: @test/warner:0.1.0\lib.typ I am emitted
#cause_warn("I am emitted")

---
#import "@test/warner:0.1.0": cause_warn
#import "@test/warner:0.1.0" as warner
#nowarn(warner)

#cause_warn("I am not emitted")

---
#import "@test/warner:0.1.0": cause_warn
#import "@test/warner:0.1.0" as warner_with_different_name
#nowarn(warner_with_different_name)

#cause_warn("I am not emitted")

---
#import "@test/second-warner:0.1.0" as unsuppressed
#import "@test/warner:0.1.0" as suppressed
#nowarn(suppressed)

#suppressed.cause_warn("I am not emitted")

// Warning: @test/second-warner:0.1.0\lib.typ I am emitted
#unsuppressed.cause_warn("I am emitted")
