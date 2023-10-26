// Test package imports
// Ref: false

---
// Test import without items.
#import "@test/adder:0.1.0"
#test(adder.add(2, 8), 10)

---
// Test import with items.
#import "@test/adder:0.1.0": add
#test(add(2, 8), 10)

---
// Test too high required compiler version.
// Error: 9-29 package requires typst 1.0.0 or newer (current version is VERSION)
#import "@test/future:0.1.0": future

---
// Error: 9-13 `@` is not a valid package namespace; expected "@{namespace}/{name}:{version}"
#import "@@": *

---
// Error: 9-16 package specification "@{namespace}/{name}:{version}" is missing name
#import "@heya": *

---
// Error: 9-15 `123` is not a valid package namespace; expected "@{namespace}/{name}:{version}"
#import "@123": *

---
// Error: 9-17 package specification "@{namespace}/{name}:{version}" is missing name
#import "@test/": *

---
// Error: 9-22 package specification "@{namespace}/{name}:{version}" is missing version
#import "@test/mypkg": *

---
// Error: 9-20 `$$$` is not a valid package name; expected "@{namespace}/{name}:{version}"
#import "@test/$$$": *

---
// Error: 9-23 package specification "@{namespace}/{name}:{version}" is missing version
#import "@test/mypkg:": *

---
// Error: 9-24 version number "{major}.{minor}.{patch}" is missing minor version
#import "@test/mypkg:0": *

---
// Error: 9-29 `latest` is not a valid major version; expected "{major}.{minor}.{patch}"
#import "@test/mypkg:latest": *

---
// Error: 9-29 `-3` is not a valid major version; expected "{major}.{minor}.{patch}"
#import "@test/mypkg:-3.0.0": *

---
// Error: 9-26 version number "{major}.{minor}.{patch}" is missing patch version
#import "@test/mypkg:0.3": *

---
// Error: 9-27 version number "{major}.{minor}.{patch}" is missing patch version
#import "@test/mypkg:0.3.": *

---
// Error: 9-28 file not found (searched at typ/compiler/#test/mypkg:1.0.0)
#import "#test/mypkg:1.0.0": *
