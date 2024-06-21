// Test decorators.

--- decorator ---

/! allow("unnecessary-stars")
#[*a*]

#{
  /! allow("unnecessary-stars")
  [*a*]
}

$
  /! allow("unnecessary-stars")
  #[*a*]
$
