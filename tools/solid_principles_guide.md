# SOLID Principles - Detailed Implementation Guide

> **Comprehensive guide with detailed examples for implementing SOLID principles in Rust and other languages.**
> 
> **For quick reference**: Use [`solid_principles_quick_reference.md`](solid_principles_quick_reference.md) for daily development.
> **For learning/deep-dives**: This guide provides complete examples and implementation strategies.

## When to Use This Guide

- **Learning SOLID**: First time implementing principles in Rust
- **Complex Refactoring**: Large architectural changes requiring detailed patterns
- **Code Review**: Need specific examples to validate design decisions
- **Debugging Design**: Understanding why current design violates principles
- **Architecture Planning**: Designing new systems following SOLID principles

## Table of Contents
1. [Overview](#overview)
2. [Single Responsibility Principle (SRP)](#single-responsibility-principle-srp)
3. [Open/Closed Principle (OCP)](#openclosed-principle-ocp)
4. [Liskov Substitution Principle (LSP)](#liskov-substitution-principle-lsp)
5. [Interface Segregation Principle (ISP)](#interface-segregation-principle-isp)
6. [Dependency Inversion Principle (DIP)](#dependency-inversion-principle-dip)
7. [AI Development Guidelines](#ai-development-guidelines)
8. [Language-Specific Examples](#language-specific-examples)

## Overview

SOLID principles are fundamental guidelines for writing maintainable, scalable software. This guide provides comprehensive implementation strategies and detailed Rust examples for AI-assisted development.

**For Quick Reference**: See [`solid_principles_quick_reference.md`](solid_principles_quick_reference.md) for essential patterns and checklists.

## Single Responsibility Principle (SRP)

> A class should have only one reason to change.

### Implementation Strategy
- **Identify responsibilities**: Each class should handle one specific concern
- **Separate concerns**: Business logic, data access, and presentation should be distinct
- **Small, focused classes**: Better than large, monolithic ones

### Good Example (Rust)
```rust
// ❌ Violates SRP - multiple responsibilities
struct UserManager {
    db_connection: String,
    email_service: String,
}

impl UserManager {
    fn save_user(&self, user: &User) {
        // database logic
    }
    
    fn send_email(&self, user: &User) {
        // email logic
    }
    
    fn validate_user(&self, user: &User) -> bool {
        // validation logic
        true
    }
}

// ✅ Follows SRP - single responsibilities
struct UserRepository {
    db_connection: String,
}

impl UserRepository {
    fn save(&self, user: &User) -> Result<(), String> {
        // only database logic
        println!("Saving user: {}", user.name);
        Ok(())
    }
}

struct EmailService {
    smtp_config: String,
}

impl EmailService {
    fn send_welcome_email(&self, user: &User) -> Result<(), String> {
        // only email logic
        println!("Sending welcome email to: {}", user.email);
        Ok(())
    }
}

struct UserValidator;

impl UserValidator {
    fn validate(&self, user: &User) -> ValidationResult {
        // only validation logic
        ValidationResult {
            is_valid: !user.name.is_empty() && user.email.contains('@'),
            errors: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct User {
    name: String,
    email: String,
}

#[derive(Debug)]
struct ValidationResult {
    is_valid: bool,
    errors: Vec<String>,
}
```

### AI Development Guidelines
- **Before adding a method**: Ask "Does this method belong in this class?"
- **When extending functionality**: Consider if a new class is needed
- **Code review prompt**: "What would cause this class to change?"

## Open/Closed Principle (OCP)

> Software entities should be open for extension, closed for modification.

### Implementation Strategy
- **Use abstractions**: Interfaces and abstract classes enable extension
- **Strategy pattern**: For varying algorithms
- **Dependency injection**: To swap implementations
- **Plugin architecture**: For extensible systems

### Good Example (Rust)
```rust
// ❌ Violates OCP - requires modification for new shapes
struct AreaCalculator;

impl AreaCalculator {
    fn calculate_area(&self, shapes: &[ShapeData]) -> f64 {
        let mut total_area = 0.0;
        for shape in shapes {
            match shape.shape_type.as_str() {
                "circle" => total_area += 3.14 * shape.radius.unwrap().powi(2),
                "rectangle" => total_area += shape.width.unwrap() * shape.height.unwrap(),
                // Adding new shape requires modifying this method
                _ => {}
            }
        }
        total_area
    }
}

#[derive(Debug)]
struct ShapeData {
    shape_type: String,
    radius: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
}

// ✅ Follows OCP - extensible without modification
trait Shape {
    fn area(&self) -> f64;
}

struct Circle {
    radius: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14 * self.radius * self.radius
    }
}

struct Rectangle {
    width: f64,
    height: f64,
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

struct AreaCalculator;

impl AreaCalculator {
    fn calculate_area(&self, shapes: &[Box<dyn Shape>]) -> f64 {
        shapes.iter().map(|shape| shape.area()).sum()
    }
}

// New shapes can be added without modifying existing code
struct Triangle {
    base: f64,
    height: f64,
}

impl Shape for Triangle {
    fn area(&self) -> f64 {
        0.5 * self.base * self.height
    }
}

// Usage example
fn example_usage() {
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { radius: 5.0 }),
        Box::new(Rectangle { width: 4.0, height: 3.0 }),
        Box::new(Triangle { base: 6.0, height: 4.0 }),
    ];
    
    let calculator = AreaCalculator;
    let total = calculator.calculate_area(&shapes);
    println!("Total area: {}", total);
}
```

### AI Development Guidelines
- **When adding new behavior**: Create new classes/modules instead of modifying existing ones
- **Design for extension**: Use interfaces and abstract base classes
- **Avoid switch statements**: Use polymorphism instead

## Liskov Substitution Principle (LSP)

> Objects of a superclass should be replaceable with objects of its subclasses without breaking functionality.

### Implementation Strategy
- **Behavioral compatibility**: Subclasses must honor the contract of their parent
- **Preconditions cannot be strengthened**: Subclasses can't be more restrictive
- **Postconditions cannot be weakened**: Subclasses must provide at least the same guarantees

### Good Example (Rust)
```rust
// ❌ Violates LSP - Square changes Rectangle behavior
#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    fn new(width: f64, height: f64) -> Self {
        Rectangle { width, height }
    }
    
    fn set_width(&mut self, width: f64) {
        self.width = width;
    }
    
    fn set_height(&mut self, height: f64) {
        self.height = height;
    }
    
    fn get_area(&self) -> f64 {
        self.width * self.height
    }
}

// This approach violates LSP if Square inherits Rectangle's behavior
struct Square {
    rectangle: Rectangle,
}

impl Square {
    fn new(side: f64) -> Self {
        Square {
            rectangle: Rectangle::new(side, side),
        }
    }
    
    fn set_width(&mut self, width: f64) {
        // This breaks LSP - changing width also changes height
        self.rectangle.width = width;
        self.rectangle.height = width; // Unexpected behavior!
    }
    
    fn set_height(&mut self, height: f64) {
        // This breaks LSP - changing height also changes width
        self.rectangle.width = height; // Unexpected behavior!
        self.rectangle.height = height;
    }
    
    fn get_area(&self) -> f64 {
        self.rectangle.get_area()
    }
}

// ✅ Follows LSP - proper abstraction using traits
trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
}

#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    fn new(width: f64, height: f64) -> Self {
        Rectangle { width, height }
    }
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
    
    fn perimeter(&self) -> f64 {
        2.0 * (self.width + self.height)
    }
}

#[derive(Debug)]
struct Square {
    side: f64,
}

impl Square {
    fn new(side: f64) -> Self {
        Square { side }
    }
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
    
    fn perimeter(&self) -> f64 {
        4.0 * self.side
    }
}

// Both can be used interchangeably as Shape
fn calculate_total_area(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

fn example_usage() {
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Rectangle::new(4.0, 5.0)),
        Box::new(Square::new(3.0)),
    ];
    
    let total = calculate_total_area(&shapes);
    println!("Total area: {}", total); // Works correctly for both
}
```

### AI Development Guidelines
- **Test substitutability**: Can you replace parent with child in all contexts?
- **Honor contracts**: Subclasses must fulfill the same promises as their parents
- **Avoid breaking changes**: Don't change expected behavior in inheritance

## Interface Segregation Principle (ISP)

> Clients should not be forced to depend on interfaces they do not use.

### Implementation Strategy
- **Small, focused interfaces**: Better than large, monolithic ones
- **Role-based interfaces**: Design around client needs
- **Composition over inheritance**: Combine multiple small interfaces

### Good Example (Rust)
```rust
// ❌ Violates ISP - fat trait forces unnecessary dependencies
trait Worker {
    fn work(&self);
    fn eat(&self);
    fn sleep(&self);
}

struct Robot {
    battery_level: u8,
}

impl Worker for Robot {
    fn work(&self) {
        println!("Robot is working with battery level: {}", self.battery_level);
    }
    
    fn eat(&self) {
        // Robot doesn't eat - this method shouldn't exist for Robot
        panic!("Robots don't eat!");
    }
    
    fn sleep(&self) {
        // Robot doesn't sleep - this method shouldn't exist for Robot
        panic!("Robots don't sleep!");
    }
}

// ✅ Follows ISP - segregated traits
trait Workable {
    fn work(&self);
}

trait Feedable {
    fn eat(&self);
}

trait Restable {
    fn sleep(&self);
}

// Additional traits for more specific behaviors
trait Rechargeable {
    fn recharge(&mut self);
    fn battery_level(&self) -> u8;
}

trait Biological {
    fn breathe(&self);
}

#[derive(Debug)]
struct Human {
    name: String,
    energy_level: u8,
}

impl Human {
    fn new(name: String) -> Self {
        Human { name, energy_level: 100 }
    }
}

impl Workable for Human {
    fn work(&self) {
        println!("{} is working with energy level: {}", self.name, self.energy_level);
    }
}

impl Feedable for Human {
    fn eat(&self) {
        println!("{} is eating to restore energy", self.name);
    }
}

impl Restable for Human {
    fn sleep(&self) {
        println!("{} is sleeping to restore energy", self.name);
    }
}

impl Biological for Human {
    fn breathe(&self) {
        println!("{} is breathing", self.name);
    }
}

#[derive(Debug)]
struct Robot {
    model: String,
    battery_level: u8,
}

impl Robot {
    fn new(model: String) -> Self {
        Robot { model, battery_level: 100 }
    }
}

impl Workable for Robot {
    fn work(&self) {
        println!("Robot {} is working with battery level: {}", self.model, self.battery_level);
    }
}

impl Rechargeable for Robot {
    fn recharge(&mut self) {
        self.battery_level = 100;
        println!("Robot {} recharged to 100%", self.model);
    }
    
    fn battery_level(&self) -> u8 {
        self.battery_level
    }
}

// Functions that depend only on what they need
fn manage_workers(workers: &[Box<dyn Workable>]) {
    for worker in workers {
        worker.work();
    }
}

fn manage_biological_entities(entities: &[Box<dyn Biological>]) {
    for entity in entities {
        entity.breathe();
    }
}

fn example_usage() {
    let workers: Vec<Box<dyn Workable>> = vec![
        Box::new(Human::new("Alice".to_string())),
        Box::new(Robot::new("R2D2".to_string())),
    ];
    
    manage_workers(&workers);
    
    // Only humans can be managed as biological entities
    let biological: Vec<Box<dyn Biological>> = vec![
        Box::new(Human::new("Bob".to_string())),
    ];
    
    manage_biological_entities(&biological);
}
```

### AI Development Guidelines
- **Design client-specific interfaces**: Focus on what clients actually need
- **Avoid God interfaces**: Split large interfaces into focused ones
- **Think in terms of roles**: What role does this client play?

## Dependency Inversion Principle (DIP)

> High-level modules should not depend on low-level modules. Both should depend on abstractions.

### Implementation Strategy
- **Dependency injection**: Inject dependencies rather than creating them
- **Inversion of control containers**: Use IoC frameworks where appropriate
- **Abstract interfaces**: Define contracts that both layers can depend on

### Good Example (Rust)
```rust
use std::fs::OpenOptions;
use std::io::Write;

// ❌ Violates DIP - high-level depends on low-level
struct FileLogger {
    file_path: String,
}

impl FileLogger {
    fn new(file_path: String) -> Self {
        FileLogger { file_path }
    }
    
    fn log(&self, message: &str) {
        println!("Writing to file {}: {}", self.file_path, message);
        // In real implementation, would write to file
    }
}

struct OrderService {
    logger: FileLogger, // Direct dependency on concrete implementation
}

impl OrderService {
    fn new() -> Self {
        // High-level module creating low-level dependency
        OrderService {
            logger: FileLogger::new("orders.log".to_string()),
        }
    }
    
    fn process_order(&self, order: &Order) {
        // Process order logic
        println!("Processing order: {}", order.id);
        
        // Tightly coupled to FileLogger - can't easily test or change
        self.logger.log(&format!("Order {} processed", order.id));
    }
}

// ✅ Follows DIP - both depend on abstraction
trait Logger {
    fn log(&self, message: &str);
    fn flush(&self) {}
}

struct FileLogger {
    file_path: String,
}

impl FileLogger {
    fn new(file_path: String) -> Self {
        FileLogger { file_path }
    }
}

impl Logger for FileLogger {
    fn log(&self, message: &str) {
        println!("File Logger - Writing to {}: {}", self.file_path, message);
        // In real implementation, would write to file
    }
    
    fn flush(&self) {
        println!("Flushing file buffer for: {}", self.file_path);
    }
}

struct DatabaseLogger {
    connection_string: String,
}

impl DatabaseLogger {
    fn new(connection_string: String) -> Self {
        DatabaseLogger { connection_string }
    }
}

impl Logger for DatabaseLogger {
    fn log(&self, message: &str) {
        println!("Database Logger - Writing to DB: {}", message);
        // In real implementation, would write to database
    }
}

struct ConsoleLogger;

impl Logger for ConsoleLogger {
    fn log(&self, message: &str) {
        println!("Console: {}", message);
    }
}

// High-level module depends on abstraction
struct OrderService<T: Logger> {
    logger: T,
    order_counter: u32,
}

impl<T: Logger> OrderService<T> {
    fn new(logger: T) -> Self {
        OrderService { 
            logger,
            order_counter: 0,
        }
    }
    
    fn process_order(&mut self, order: &Order) -> Result<String, String> {
        // Validate order
        if order.amount <= 0.0 {
            let error_msg = format!("Invalid order amount: {}", order.amount);
            self.logger.log(&format!("ERROR: {}", error_msg));
            return Err(error_msg);
        }
        
        // Process order logic
        self.order_counter += 1;
        let confirmation = format!("CONF-{:06}", self.order_counter);
        
        // Log through abstraction - doesn't know or care about implementation
        self.logger.log(&format!("Order {} processed successfully. Confirmation: {}", order.id, confirmation));
        
        Ok(confirmation)
    }
    
    fn get_order_stats(&self) -> u32 {
        self.order_counter
    }
}

// Domain model
#[derive(Debug)]
struct Order {
    id: String,
    amount: f64,
    customer_id: String,
}

impl Order {
    fn new(id: String, amount: f64, customer_id: String) -> Self {
        Order { id, amount, customer_id }
    }
}

// Dependency injection through constructor
fn example_usage() {
    // Can easily swap logger implementations
    let file_logger = FileLogger::new("orders.log".to_string());
    let mut order_service = OrderService::new(file_logger);
    
    let order1 = Order::new("ORD-001".to_string(), 100.50, "CUST-123".to_string());
    match order_service.process_order(&order1) {
        Ok(confirmation) => println!("Order confirmed: {}", confirmation),
        Err(error) => println!("Order failed: {}", error),
    }
    
    // Easy to switch to different logger without changing OrderService
    let db_logger = DatabaseLogger::new("postgres://localhost:5432/orders".to_string());
    let mut db_order_service = OrderService::new(db_logger);
    
    let order2 = Order::new("ORD-002".to_string(), 250.75, "CUST-456".to_string());
    db_order_service.process_order(&order2);
    
    // Console logger for development
    let console_logger = ConsoleLogger;
    let mut dev_service = OrderService::new(console_logger);
    
    let order3 = Order::new("ORD-003".to_string(), -10.0, "CUST-789".to_string());
    dev_service.process_order(&order3); // Will log error
}

// Easy testing with mock logger
#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockLogger {
        logs: std::cell::RefCell<Vec<String>>,
    }
    
    impl MockLogger {
        fn new() -> Self {
            MockLogger {
                logs: std::cell::RefCell::new(Vec::new()),
            }
        }
        
        fn get_logs(&self) -> Vec<String> {
            self.logs.borrow().clone()
        }
    }
    
    impl Logger for MockLogger {
        fn log(&self, message: &str) {
            self.logs.borrow_mut().push(message.to_string());
        }
    }
    
    #[test]
    fn test_order_processing_success() {
        let mock_logger = MockLogger::new();
        let mut service = OrderService::new(mock_logger);
        
        let order = Order::new("TEST-001".to_string(), 100.0, "CUST-001".to_string());
        let result = service.process_order(&order);
        
        assert!(result.is_ok());
        let logs = service.logger.get_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("TEST-001"));
        assert!(logs[0].contains("processed successfully"));
    }
    
    #[test]
    fn test_order_processing_failure() {
        let mock_logger = MockLogger::new();
        let mut service = OrderService::new(mock_logger);
        
        let invalid_order = Order::new("TEST-002".to_string(), -50.0, "CUST-002".to_string());
        let result = service.process_order(&invalid_order);
        
        assert!(result.is_err());
        let logs = service.logger.get_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("ERROR"));
    }
}
```

### AI Development Guidelines
- **Inject, don't create**: Pass dependencies as parameters
- **Define interfaces first**: Think about what you need, not how it's implemented
- **Test with mocks**: DIP makes testing much easier

## AI Development Guidelines

### Before Writing Code
1. **Identify the single responsibility** - What is this class/module supposed to do?
2. **Design for extension** - How might this need to change in the future?
3. **Define interfaces** - What contract should this fulfill?
4. **Consider dependencies** - What does this need from other components?

### During Implementation
1. **Keep methods small** - Each method should have one clear purpose
2. **Use descriptive names** - Make the responsibility obvious from the name
3. **Avoid deep inheritance** - Prefer composition over inheritance
4. **Inject dependencies** - Don't create dependencies inside classes

### Code Review Checklist
- [ ] Does each class have a single, clear responsibility?
- [ ] Can new behavior be added without modifying existing code?
- [ ] Are subclasses truly substitutable for their parents?
- [ ] Are interfaces focused and client-specific?
- [ ] Do high-level modules avoid depending on implementation details?

### Refactoring Signals
- **Large classes** → Split by responsibility (SRP)
- **Switch statements** → Use polymorphism (OCP)
- **Type checking in client code** → Fix inheritance hierarchy (LSP)
- **Empty method implementations** → Segregate interfaces (ISP)
- **Direct instantiation of dependencies** → Use dependency injection (DIP)

## Language-Specific Examples

### Python
- **SRP**: Use modules and classes appropriately
- **OCP**: Leverage duck typing and protocols
- **LSP**: Be careful with method overrides
- **ISP**: Use Protocol classes (Python 3.8+)
- **DIP**: Use dependency injection frameworks like `dependency-injector`

### TypeScript/JavaScript
- **SRP**: Use ES6 modules and classes
- **OCP**: Leverage interfaces and function composition
- **LSP**: Maintain behavioral contracts in inheritance
- **ISP**: Create focused interface definitions
- **DIP**: Use constructor injection and IoC containers

### Java
- **SRP**: Single-purpose classes and packages
- **OCP**: Abstract classes and interfaces
- **LSP**: Proper inheritance hierarchies
- **ISP**: Multiple small interfaces
- **DIP**: Spring Framework dependency injection

### C#
- **SRP**: Classes with single concerns
- **OCP**: Interfaces and virtual methods
- **LSP**: Proper inheritance design
- **ISP**: Role-based interfaces
- **DIP**: Built-in dependency injection container

### Rust
- **SRP**: Modules and structs with clear purposes; use `impl` blocks to group related functionality
- **OCP**: Traits for abstraction, generics for type flexibility, enum variants for extensible behavior
- **LSP**: Careful trait implementation ensuring behavioral contracts; avoid panicking in trait methods
- **ISP**: Small, focused traits; use trait composition rather than large trait hierarchies
- **DIP**: Trait objects (`Box<dyn Trait>`) and generic parameters for dependency injection; constructor injection pattern

## Common Violations and Solutions

### SRP Violations
- **Problem**: God classes doing everything
- **Solution**: Extract related methods into separate classes

### OCP Violations
- **Problem**: Modifying existing code for new features
- **Solution**: Use strategy pattern or plugin architecture

### LSP Violations
- **Problem**: Subclasses throwing exceptions for inherited methods
- **Solution**: Redesign inheritance hierarchy or use composition

### ISP Violations
- **Problem**: Interfaces with many unrelated methods
- **Solution**: Break into smaller, role-specific interfaces

### DIP Violations
- **Problem**: Classes creating their own dependencies
- **Solution**: Use constructor injection, trait objects, or dependency injection containers

## Rust-Specific SOLID Implementation Patterns

### Ownership and SOLID Principles

Rust's ownership system naturally enforces some SOLID principles:

```rust
// SRP: Clear ownership boundaries
struct DatabaseConfig {
    url: String,
    timeout: u64,
}

struct Database {
    config: DatabaseConfig,  // Owned configuration
}

impl Database {
    fn new(config: DatabaseConfig) -> Self {
        Database { config }
    }
}

// OCP: Using trait objects for runtime polymorphism
trait PaymentProcessor {
    fn process_payment(&self, amount: f64) -> Result<String, String>;
}

struct PaymentService {
    processors: Vec<Box<dyn PaymentProcessor>>,
}

impl PaymentService {
    fn add_processor(&mut self, processor: Box<dyn PaymentProcessor>) {
        self.processors.push(processor);
    }
}
```

### Error Handling and SOLID

Rust's `Result` type naturally supports LSP:

```rust
trait DataStore {
    type Error;
    fn save(&self, data: &str) -> Result<(), Self::Error>;
    fn load(&self, id: &str) -> Result<String, Self::Error>;
}

// All implementations must return Results, maintaining behavioral compatibility
struct FileStore;
struct DatabaseStore;

impl DataStore for FileStore {
    type Error = std::io::Error;
    
    fn save(&self, data: &str) -> Result<(), Self::Error> {
        // Implementation that properly handles errors
        Ok(())
    }
    
    fn load(&self, id: &str) -> Result<String, Self::Error> {
        Ok(format!("loaded-{}", id))
    }
}
```

### Dependency Injection with Lifetimes

```rust
// Advanced DIP with lifetimes for borrowed dependencies
struct Service<'a, L: Logger> {
    logger: &'a L,
    name: String,
}

impl<'a, L: Logger> Service<'a, L> {
    fn new(logger: &'a L, name: String) -> Self {
        Service { logger, name }
    }
    
    fn do_work(&self) -> Result<(), String> {
        self.logger.log(&format!("Service {} starting work", self.name));
        // Work logic here
        self.logger.log(&format!("Service {} completed work", self.name));
        Ok(())
    }
}
```

## Conclusion

SOLID principles are not just theoretical concepts—they're practical tools for creating maintainable, testable, and extensible software. When working with AI assistants:

1. **Start with principles in mind**: Design following SOLID from the beginning
2. **Refactor incrementally**: Apply principles gradually to existing code
3. **Use principles as review criteria**: Check each principle during code review
4. **Document architectural decisions**: Record why you chose specific patterns

Remember: These principles guide good design but shouldn't be followed blindly. Use judgment to apply them appropriately to your specific context and requirements.