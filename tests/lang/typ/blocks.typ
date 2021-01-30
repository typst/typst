// Empty.
{}

// Basic expression.
{1}

// Bad expression.
// Error: 2-4 expected expression, found invalid token
{1u}

// Missing closing brace in nested block.
// Error: 5-5 expected closing brace
{({1) + 1}

// Missing closing bracket in template expression.
// Error: 11-11 expected closing bracket
{[_] + [3_}
