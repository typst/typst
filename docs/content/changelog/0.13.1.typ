#import "utils.typ": *

= Command Line Interface <command-line-interface>
- Fixed high CPU usage for `typst watch` on Linux. Depending on the project size, CPU usage would spike for varying amounts of time. This bug appeared with 0.13.0 due to a behavioral change in the inotify file watching backend.

= HTML export <html-export>
- Fixed export of tables with @table.gutter[gutters]
- Fixed usage of `<html>` and `<body>` element within @reference:context[context]
- Fixed querying of @metadata[metadata] next to `<html>` and `<body>` element

= Visualization <visualization>
- Fixed @curve[curves] with multiple non-closed components

= Introspection <introspection>
- Fixed a regression where labelled @symbol[symbols] could not be @query[queried] by label

= Deprecations <deprecations>
- Fixed false positives in deprecation warnings for type/str comparisons
