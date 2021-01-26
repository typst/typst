// Basic expression.
{1}

// Error: 1:2-1:2 expected expression
{}

// Bad expression.
// Error: 1:2-1:4 expected expression, found invalid token
{1u}

// Two expressions are not allowed.
// Error: 1:4-1:5 unexpected integer
{2 3}

// Missing closing brace in nested block.
// Error: 1:5-1:5 expected closing brace
{({1) + 2}

// Missing closing bracket in template expression.
// Error: 1:11-1:11 expected closing bracket
{[_] + [4_}
