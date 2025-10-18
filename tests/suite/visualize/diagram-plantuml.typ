// Test PlantUML diagram rendering

--- plantuml-sequence ---
// Sequence diagram (renders as placeholder SVG)
#diagram(
  kind: "plantuml",
  ```
  @startuml
  Alice -> Bob: Authentication Request
  Bob --> Alice: Authentication Response
  
  Alice -> Bob: Another authentication Request
  Alice <-- Bob: Another authentication Response
  @enduml
  ```
)

--- plantuml-usecase ---
// Use case diagram (renders as placeholder SVG)
#diagram(
  kind: "plantuml",
  width: 70%,
  ```
  @startuml
  left to right direction
  actor Guest as g
  package Professional {
    actor Chef as c
    actor "Food Critic" as fc
  }
  package Restaurant {
    usecase "Eat Food" as UC1
    usecase "Pay for Food" as UC2
    usecase "Drink" as UC3
    usecase "Review" as UC4
  }
  g --> UC1
  g --> UC2
  g --> UC3
  fc --> UC4
  c --> UC1
  @enduml
  ```
)

--- plantuml-class ---
// Class diagram
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

--- plantuml-activity ---
// Activity diagram
#diagram(
  kind: "plantuml",
  ```
  @startuml
  start
  :Read data;
  if (data valid?) then (yes)
    :Process data;
  else (no)
    :Show error;
    stop
  endif
  :Save results;
  stop
  @enduml
  ```
)

--- plantuml-component ---
// Component diagram
#diagram(
  kind: "plantuml",
  ```
  @startuml
  package "Some Group" {
    HTTP - [First Component]
    [Another Component]
  }
  
  node "Other Groups" {
    FTP - [Second Component]
    [First Component] --> FTP
  }
  @enduml
  ```
)

---
// Object diagram
#diagram(
  kind: "plantuml",
  ```
  @startuml
  object "Order #12345" as order {
    id = 12345
    date = 2024-01-15
    status = "Delivered"
    amount = 15000
  }
  
  object "Customer" as customer {
    id = 789
    name = "John Doe"
    email = "john@example.com"
  }
  
  order --> customer
  @enduml
  ```
)