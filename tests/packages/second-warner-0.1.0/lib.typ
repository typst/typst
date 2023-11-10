#let cause_warn(message) = {
  // The newlines before this is by design, as the test utility only supports annotating lines, not files.
  // It must match the offset inside the test case starting after ---.
  warn(message)
}
