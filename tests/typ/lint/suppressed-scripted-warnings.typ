// Test suppression of warnings in transient files.
// Ref: false
// ValidateTransientDiagnostics: true

---
#import "@test/warner:0.1.0": cause_warn
#import "@test/warner:0.1.0" as warner
#nowarn(warner)

#cause_warn("Hi there")
