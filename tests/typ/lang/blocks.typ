{1}

// Function calls.
{f(1)}
{[[f 1]]}

// Error: 1:2-1:2 expected expression
{}

// Error: 1:2-1:4 expected expression, found invalid token
{1u}

// Error: 1:5-1:5 expected closing brace
{({1) + 2}

// Error: 1:12-1:12 expected closing bracket
{[*] + [ok*}

// Error: 2:4-2:5 unexpected hex value
// Error: 1:5-1:6 unexpected opening brace
{1 #{} _end_
