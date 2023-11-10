// Test suppression of warnings in transient files.
// Ref: false
// ValidateTransientDiagnostics: true
---
// Warning: 6-13 I am emitted
#let test_helper = 1
// ValidateTransientDiagnostics should make it possible to test this behaviour, so let's make sure that actually works.
// Due to ValidateTransientDiagnostics, the location is not matching the location of the method call that causes the diagnostic,
// instead it matches the location in the (package) file that causes the diagnostic.
#import "@test/warner:0.1.0": cause_warn

#cause_warn("I am emitted")

---
#import "@test/warner:0.1.0": cause_warn
#import "@test/warner:0.1.0" as warner
#nowarn(warner)

#cause_warn("I am not emitted")

---
#import "@test/second-warner:0.1.0" as suppressed
#import "@test/warner:0.1.0" as unsuppressed

#nowarn(suppressed)

suppressed.cause_warn("I am not emitted")
// Warning: 7-40 I am emitted
unsuppressed.cause_warn("I am emitted")