---
description: |
  Learn how to create diagrams using Mermaid and PlantUML syntax directly in your Typst documents. This guide covers installation, basic usage, examples, and troubleshooting.
---

# Creating Diagrams

Typst supports creating diagrams using Mermaid and PlantUML syntax directly in your documents.

## Overview

The `diagram` function allows you to create various types of diagrams using popular diagramming syntaxes:

- **Mermaid**: Flowcharts, sequence diagrams, class diagrams, Gantt charts, and more
- **PlantUML**: Sequence diagrams, use case diagrams, class diagrams, activity diagrams, and more

## Current Status

The diagram functionality is currently in development. It provides placeholder SVGs for both Mermaid and PlantUML diagrams. Full rendering support will be added in future versions.

## Future Installation Requirements

When full support is implemented, you will need:

### Mermaid CLI

For Mermaid diagrams, install the Mermaid CLI:

```bash
npm install -g @mermaid-js/mermaid-cli
```

### PlantUML

For PlantUML diagrams, install PlantUML:

**macOS (Homebrew):**
```bash
brew install plantuml
```

**Ubuntu/Debian:**
```bash
sudo apt-get install plantuml
```

**Windows (Chocolatey):**
```bash
choco install plantuml
```

## Basic Usage

### Syntax

```typst
#diagram(
  kind: "mermaid" | "plantuml",
  source: string,
  width: auto | length,
  height: auto | length,
  alt: string
)
```

### Parameters

- **kind** (required): Diagram type - `"mermaid"` or `"plantuml"`
- **source** (required): Diagram source code as a string
- **width** (optional): Diagram width (default: auto)
- **height** (optional): Diagram height (default: auto)
- **alt** (optional): Alternative text for accessibility

## Examples

**Note**: The following examples will currently render as placeholder SVGs. Full diagram rendering will be available in future versions.

### Mermaid Examples

#### Flowchart

```typst
#diagram(
  kind: "mermaid",
  ```
  graph TD
    A[Start] --> B{Check condition}
    B -->|Yes| C[Success]
    B -->|No| D[Error]
  ```
)
```

### Sequence Diagram

```typst
#diagram(
  kind: "mermaid",
  width: 80%,
  ```
  sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hello Bob!
    B-->>A: Hello Alice!
  ```
)
```

### Class Diagram

```typst
#diagram(
  kind: "mermaid",
  ```
  classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
  ```
)
```

### PlantUML Examples

#### Sequence Diagram

```typst
#diagram(
  kind: "plantuml",
  ```
  @startuml
  Alice -> Bob: Authentication Request
  Bob --> Alice: Authentication Response
  @enduml
  ```
)
```

### Use Case Diagram

```typst
#diagram(
  kind: "plantuml",
  width: 70%,
  ```
  @startuml
  left to right direction
  actor Client as client
  actor Admin as admin
  
  package System {
    usecase "View Products" as UC1
    usecase "Place Order" as UC2
    usecase "Manage Products" as UC3
  }
  
  client --> UC1
  client --> UC2
  admin --> UC3
  @enduml
  ```
)
```

### Class Diagram

```typst
#diagram(
  kind: "plantuml",
  ```
  @startuml
  class Car
  
  class Driver {
    +name: string
    +age: int
    +drive()
  }
  
  class License {
    +number: string
    +expiry: date
  }
  
  Driver "1" *-- "1" License
  Driver "1" o-- "many" Car
  @enduml
  ```
)
```

## Using with Figures

Diagrams can be wrapped in `figure` to add captions and numbering:

```typst
#figure(
  diagram(
    kind: "mermaid",
    alt: "Data processing workflow",
    ```
    graph LR
      A[Input] --> B[Process]
      B --> C[Output]
    ```
  ),
  caption: [Data processing workflow],
)
```

## Tips and Best Practices

1. **Accessibility**: Always specify the `alt` parameter to ensure document accessibility
2. **Sizing**: Use `width` and `height` parameters to control diagram size
3. **Figures**: Wrap diagrams in `figure` for automatic numbering and captions
4. **Performance**: Consider caching complex diagrams as SVG files for faster compilation
5. **Testing**: Verify your diagram syntax using online editors before including in documents

## Troubleshooting

### Current Limitations

Since the diagram functionality is in development:
- All diagrams currently render as placeholder SVGs
- No external tools are required at this time
- Full rendering support will be added in future versions

### Future Troubleshooting

When full support is implemented, common issues may include:

#### Error: "mermaid-cli (mmdc) not found"
Make sure Mermaid CLI is installed:
```bash
npm install -g @mermaid-js/mermaid-cli
```

#### Error: "plantuml not found"
Make sure PlantUML is installed and available:
```bash
plantuml -version
```

#### Encoding Issues
If you have problems with Cyrillic or other characters:
- Ensure the Typst file is saved in UTF-8
- Install required fonts for PlantUML

## Current Limitations

- Diagrams are rendered as placeholder SVGs
- Full diagram rendering is not yet implemented
- Interactive diagram features are not supported
- Rendering happens during compilation, not in real-time

## Future Limitations

When full support is implemented:
- Diagrams will be rendered in SVG format
- Will require installation of external tools (mmdc, plantuml)

## Resources

- [Mermaid Documentation](https://mermaid.js.org/)
- [PlantUML Documentation](https://plantuml.com/)
- [Mermaid Live Editor](https://mermaid.live/)
- [PlantUML Server](https://www.plantuml.com/plantuml/)
