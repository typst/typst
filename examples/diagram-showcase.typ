// Demonstration of Mermaid and PlantUML diagram support in Typst

#set page(width: 21cm, height: 29.7cm, margin: 2cm)
#set text(font: "Liberation Sans", size: 11pt)
#set heading(numbering: "1.")

= Diagram Support in Typst

This document demonstrates the new functionality for creating Mermaid and PlantUML diagrams directly in Typst documents.

== Mermaid Diagrams

=== Flowchart

#figure(
  diagram(
    kind: "mermaid",
    width: 80%,
    alt: "Decision process flowchart",
    ```
    graph TD
      A[Start] --> B{Condition met?}
      B -->|Yes| C[Execute Action A]
      B -->|No| D[Execute Action B]
      C --> E[End]
      D --> E
      style A fill:#90EE90
      style E fill:#FFB6C1
      style B fill:#87CEEB
    ```
  ),
  caption: [Example flowchart using Mermaid],
)

=== Sequence Diagram

#figure(
  diagram(
    kind: "mermaid",
    width: 90%,
    alt: "User-system interaction sequence diagram",
    ```
    sequenceDiagram
      participant User as User
      participant Web as Web Server
      participant DB as Database
      
      User->>Web: HTTP GET /data
      activate Web
      Web->>DB: SELECT * FROM table
      activate DB
      DB-->>Web: Results
      deactivate DB
      Web-->>User: JSON response
      deactivate Web
    ```
  ),
  caption: [Data request sequence diagram],
)

=== Class Diagram

#figure(
  diagram(
    kind: "mermaid",
    alt: "Animal hierarchy class diagram",
    ```
    classDiagram
      Animal <|-- Mammal
      Animal <|-- Bird
      Mammal <|-- Dog
      Mammal <|-- Cat
      Bird <|-- Eagle
      
      class Animal{
        +String name
        +int age
        +makeSound()
        +move()
      }
      
      class Mammal{
        +int furLength
        +giveBirth()
      }
      
      class Bird{
        +float wingspan
        +layEggs()
      }
      
      class Dog{
        +String breed
        +bark()
      }
    ```
  ),
  caption: [Class hierarchy for animal representation],
)

=== State Diagram

#figure(
  diagram(
    kind: "mermaid",
    width: 70%,
    alt: "Order state diagram",
    ```
    stateDiagram-v2
      [*] --> Created
      Created --> Paid: payment
      Created --> Cancelled: cancel
      Paid --> Shipped: shipping
      Shipped --> Delivered: delivery
      Delivered --> [*]
      Cancelled --> [*]
      
      note right of Created
        New order
        awaiting payment
      end note
    ```
  ),
  caption: [Order states in an e-commerce system],
)

=== Gantt Chart

#figure(
  diagram(
    kind: "mermaid",
    width: 100%,
    alt: "Software development project Gantt chart",
    ```
    gantt
      title Development Project Schedule
      dateFormat YYYY-MM-DD
      
      section Analysis
        Requirements gathering      :a1, 2024-01-01, 14d
        Requirements analysis      :a2, after a1, 10d
      
      section Design
        Architecture              :d1, after a2, 15d
        UI/UX Design              :d2, after a2, 20d
      
      section Development
        Backend                   :dev1, after d1, 30d
        Frontend                  :dev2, after d2, 30d
      
      section Testing
        Unit Testing              :t1, after dev1, 10d
        Integration Testing       :t2, after t1, 10d
        Acceptance Testing        :t3, after t2, 5d
    ```
  ),
  caption: [Software development project timeline],
)

#pagebreak()

== PlantUML Diagrams

=== Use Case Diagram

#figure(
  diagram(
    kind: "plantuml",
    width: 80%,
    alt: "Library management system use case diagram",
    ```
    @startuml
    left to right direction
    
    actor "Reader" as reader
    actor "Librarian" as librarian
    actor "Administrator" as admin
    
    rectangle "Library System" {
      usecase "Search Books" as UC1
      usecase "Reserve Book" as UC2
      usecase "Checkout Book" as UC3
      usecase "Return Book" as UC4
      usecase "Manage Catalog" as UC5
      usecase "Manage Users" as UC6
    }
    
    reader --> UC1
    reader --> UC2
    librarian --> UC3
    librarian --> UC4
    librarian --> UC5
    admin --> UC6
    
    UC3 ..> UC1 : <<include>>
    UC4 ..> UC1 : <<include>>
    @enduml
    ```
  ),
  caption: [Library management system use cases],
)

=== Component Diagram

#figure(
  diagram(
    kind: "plantuml",
    alt: "Web application component diagram",
    ```
    @startuml
    package "Frontend" {
      [React App]
      [Redux Store]
    }
    
    package "Backend" {
      [API Gateway]
      [Auth Service]
      [User Service]
      [Order Service]
    }
    
    database "PostgreSQL" as db
    queue "RabbitMQ" as mq
    
    [React App] --> [Redux Store]
    [React App] --> [API Gateway] : HTTPS
    [API Gateway] --> [Auth Service]
    [API Gateway] --> [User Service]
    [API Gateway] --> [Order Service]
    
    [Auth Service] --> db
    [User Service] --> db
    [Order Service] --> db
    [Order Service] --> mq
    @enduml
    ```
  ),
  caption: [Microservices web application architecture],
)

=== Deployment Diagram

#figure(
  diagram(
    kind: "plantuml",
    width: 90%,
    alt: "System deployment diagram",
    ```
    @startuml
    node "Web Server" {
      component [Nginx]
      component [React App]
    }
    
    node "Application Server" {
      component [Node.js]
      component [API]
    }
    
    node "Database Server" {
      database [PostgreSQL]
    }
    
    node "Cache Server" {
      component [Redis]
    }
    
    [Nginx] --> [React App]
    [React App] --> [API] : REST
    [API] --> [PostgreSQL] : SQL
    [API] --> [Redis] : Cache
    @enduml
    ```
  ),
  caption: [Application deployment architecture],
)

=== Activity Diagram

#figure(
  diagram(
    kind: "plantuml",
    width: 60%,
    alt: "User registration process activity diagram",
    ```
    @startuml
    start
    
    :User opens registration form;
    :Enter data;
    
    if (Data valid?) then (yes)
      :Check email;
      if (Email already exists?) then (yes)
        :Show error;
        stop
      else (no)
        :Create account;
        :Send confirmation email;
        :Show success message;
        stop
      endif
    else (no)
      :Show validation errors;
      stop
    endif
    
    @enduml
    ```
  ),
  caption: [User registration process],
)

=== Object Diagram

#figure(
  diagram(
    kind: "plantuml",
    alt: "Order system object diagram",
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
    
    object "Product #1" as item1 {
      id = 101
      name = "Laptop"
      price = 12000
      quantity = 1
    }
    
    object "Product #2" as item2 {
      id = 102
      name = "Mouse"
      price = 3000
      quantity = 1
    }
    
    order --> customer
    order --> item1
    order --> item2
    @enduml
    ```
  ),
  caption: [Object instances in the order system],
)

== Conclusion

The new Mermaid and PlantUML diagram support significantly expands Typst's capabilities for creating technical documentation, scientific papers, and other documents requiring data and process visualization.

=== Advantages

- *Built-in support*: Diagrams are created directly in the document code
- *Version control*: Diagrams are stored as text and easily tracked in Git
- *Consistency*: Diagrams automatically update when code changes
- *Flexibility*: Support for multiple diagram types for various tasks

=== Usage Recommendations

+ Use the `width` parameter to control diagram size
+ Always specify `alt` text for accessibility
+ Wrap diagrams in `figure` for automatic numbering
+ Use captions to explain diagrams

=== Best Practices

1. **Performance**: Consider caching complex diagrams as SVG files
2. **Accessibility**: Always provide alternative text descriptions
3. **Consistency**: Use consistent styling across all diagrams
4. **Documentation**: Include captions and explanations for complex diagrams

=== Future Enhancements

Planned improvements include:
- Diagram caching for faster compilation
- Support for additional diagram formats (GraphViz, D2)
- Built-in rendering without external dependencies
- Interactive diagrams for HTML export
- Custom themes and styling options