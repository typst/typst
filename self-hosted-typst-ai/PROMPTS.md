# AI Prompt Examples for Typst Document Generation

Use these prompts with Continue.dev chat or custom commands to generate Typst content.

## Document Structure

### Create a Report
```
Create a professional report with:
- Title: "Q4 Sales Analysis"
- Author: John Smith
- A4 paper with 2cm margins
- Numbered headings
- Page numbers in footer
- Sections: Executive Summary, Market Overview, Sales Data, Conclusions
```

### Academic Paper
```
Create an academic paper template for a computer science research paper with:
- Two-column layout
- Abstract section
- ACM-style formatting
- Code listing support
- Bibliography section at the end
```

### Letter Template
```
Create a formal business letter template with:
- Company letterhead area
- Date aligned right
- Recipient address block
- Subject line
- Professional closing
```

## Tables

### Simple Table
```
Add a table with columns: Product Name, SKU, Price, Stock
Include 5 rows of sample electronics data
```

### Styled Table
```
Create a table showing quarterly revenue by region:
- Columns: Region, Q1, Q2, Q3, Q4, Total
- Rows: North, South, East, West
- Use alternating row colors
- Bold the header and Total column
- Right-align all numbers
```

### Comparison Table
```
Create a feature comparison table for three products:
- Rows: Price, Speed, Storage, Support, Rating
- Use checkmarks and X marks for boolean features
- Highlight the recommended option
```

## Math & Equations

### Single Equation
```
Add the quadratic formula with proper mathematical notation
```

### Equation Block
```
Add a numbered equation block showing:
1. The definition of a derivative
2. The chain rule
3. Integration by parts
```

### Inline Math
```
Write a paragraph explaining that the area of a circle is pi times r squared, with the formula inline
```

## Lists & Content

### Bullet List
```
Create a bullet list of 5 project milestones with sub-items for each
```

### Numbered Steps
```
Add numbered steps for installing Docker on Ubuntu:
1. Update packages
2. Install dependencies
3. Add Docker repository
4. Install Docker
5. Verify installation
```

### Definition List
```
Create a glossary section with definitions for:
- API
- REST
- JSON
- OAuth
```

## Code & Technical

### Code Block
```
Add a Python code example showing a function that:
- Takes a list of numbers
- Filters out negative numbers
- Returns the sum of remaining numbers
Include type hints and a docstring
```

### Multiple Languages
```
Show the same "Hello World" program in:
- Python
- JavaScript
- Rust
- Go
Each in its own code block with syntax highlighting
```

## Figures & Images

### Figure Placeholder
```
Add a figure placeholder for a chart showing "Monthly Active Users 2024"
Include a descriptive caption and figure number
```

### Multiple Figures
```
Create a figure grid with 2x2 placeholders for:
- System Architecture
- Data Flow Diagram
- User Interface Mockup
- Deployment Topology
```

## Complex Layouts

### Two-Column Section
```
Create a section with two columns:
- Left: Project description text
- Right: Key statistics in a box
```

### Sidebar
```
Add a colored sidebar box containing:
- A tip or note icon
- Warning text about common mistakes
- Styled with a yellow background
```

### Timeline
```
Create a project timeline showing:
- Phase 1: Research (Jan-Feb)
- Phase 2: Design (Mar-Apr)
- Phase 3: Development (May-Aug)
- Phase 4: Testing (Sep-Oct)
- Phase 5: Launch (Nov)
```

## Document Conversion

### LaTeX to Typst
```
Convert this LaTeX to Typst:

\begin{equation}
\int_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
\end{equation}

\begin{table}[h]
\centering
\begin{tabular}{|l|c|r|}
\hline
Name & Age & Score \\
\hline
Alice & 25 & 95 \\
Bob & 30 & 87 \\
\hline
\end{tabular}
\caption{Student Scores}
\end{table}
```

### Markdown to Typst
```
Convert this Markdown structure to proper Typst:

# Main Title

## Introduction

Some **bold** and *italic* text.

- Item 1
- Item 2
  - Sub-item

| Col1 | Col2 |
|------|------|
| A    | B    |
```

## Fixing Errors

### Syntax Fix
```
Fix this Typst code that's giving errors:

#set page(paper: a4)  // Error: expected string
#table(
  column: 3,  // Error: unknown parameter
  [A], [B], [C]
)
```

### Style Improvement
```
Improve this Typst code to be more idiomatic:

#text(size: 20pt)[#text(weight: "bold")[Title]]
#v(10pt)
#text(size: 12pt)[By Author Name]
```

## Tips for Better Results

1. **Be Specific**: Include exact column names, sample data types, and styling preferences
2. **Mention Packages**: If you want a specific package used, name it (e.g., "use the tablex package")
3. **Describe Layout**: Specify alignment, spacing, colors, and fonts
4. **Include Context**: Mention if this is for academic, business, or personal use
5. **Iterate**: Ask follow-up questions to refine the output
