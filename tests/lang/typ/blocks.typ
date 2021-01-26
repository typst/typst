// Empty.
{}

// Basic expression.
{1}

// Bad expression.
// Error: 1:2-1:4 expected expression, found invalid token
{1u}

// Missing closing brace in nested block.
// Error: 1:5-1:5 expected closing brace
{({1) + 1}

// Missing closing bracket in template expression.
// Error: 1:11-1:11 expected closing bracket
{[_] + [3_}
