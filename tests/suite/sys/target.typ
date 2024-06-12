--- success : PDF ---
#if sys.target == "pdf" {
  [Hello World]
} else {
  panic("This will never happen")
}

--- failure : PDF ---
#if sys.target == "pdf" {
  // Error: 3-18 panicked with: "Whoops"
  panic("Whoops")
}
