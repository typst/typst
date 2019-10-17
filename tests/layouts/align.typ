{size:150pt*208pt}

// Without newline in between
[align: left][Left: {lorem:20}]
[align: right][Right: {lorem:20}]

// With newline in between
[align: center][Center: {lorem:80}]

[align: left][Left: {lorem:20}]

// Context-modifying align
[align: right]

New Right: {lorem:30}

[align: left][Inside Left: {lorem:10}]

Right Again: {lorem:10}

// Reset context-modifier
[align: left]

// All in one line
{lorem:25} [align: right][{lorem:50}] {lorem:15}
