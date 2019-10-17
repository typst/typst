{size:150pt*215pt}

// ---------------------------------- //
// Without newline in between.
[align: left][Left: {lorem:20}]
[align: right][Right: {lorem:20}]

// Over three pages.
[align: center][Center: {lorem:80}]

// Over multiple pages after the pervious 3-page run.
[align: left][Left: {lorem:80}]

[pagebreak]

// ---------------------------------- //
// Context-modifying align.
[align: right]

Context Right: {lorem:10}

[align: left][In-between Left: {lorem:10}]

Right Again: {lorem:10}

// Reset context-modifier.
[align: left]

[pagebreak]

// ---------------------------------- //
// All in one line.
All in one line: {lorem:25} [align: right][{lorem:50}] {lorem:15}
