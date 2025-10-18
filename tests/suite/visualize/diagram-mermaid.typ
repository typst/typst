// Test Mermaid diagram rendering

--- mermaid-flowchart ---
// Simple flowchart (will fail without mmdc, but tests syntax)
#diagram(
  kind: "mermaid",
  ```
  graph TD
    A[Start] --> B{Is it?}
    B -->|Yes| C[OK]
    B -->|No| D[End]
  ```
)

--- mermaid-sequence ---
// Sequence diagram
#diagram(
  kind: "mermaid",
  width: 80%,
  ```
  sequenceDiagram
    Alice->>John: Hello John, how are you?
    John-->>Alice: Great!
    Alice-)John: See you later!
  ```
)

--- mermaid-class ---
// Class diagram
#diagram(
  kind: "mermaid",
  ```
  classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
      +String beakColor
      +swim()
      +quack()
    }
  ```
)

--- mermaid-gantt ---
// Gantt chart
#diagram(
  kind: "mermaid",
  ```
  gantt
    title A Gantt Diagram
    dateFormat YYYY-MM-DD
    section Section
      A task           :a1, 2024-01-01, 30d
      Another task     :after a1, 20d
  ```
)

--- mermaid-state ---
// State diagram
#diagram(
  kind: "mermaid",
  ```
  stateDiagram-v2
    [*] --> Still
    Still --> [*]
    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
  ```
)

--- mermaid-er ---
// ER diagram
#diagram(
  kind: "mermaid",
  ```
  erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    CUSTOMER }|..|{ DELIVERY-ADDRESS : uses
  ```
)