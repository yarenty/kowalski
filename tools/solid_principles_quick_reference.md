# SOLID Principles Quick Reference

> **Quick reference for SOLID principles in AI-assisted development. For detailed examples see [`solid_principles_guide.md`](solid_principles_guide.md).**

## The Five Principles

| Principle | Definition | Key Question | AI Guideline |
|-----------|------------|--------------|--------------|
| **S**RP | Single Responsibility | Does this class have one reason to change? | Before adding functionality, ask "Does this belong here?" |
| **O**CP | Open/Closed | Can I extend behavior without modifying existing code? | Extend through new classes/modules, not modifications |
| **L**SP | Liskov Substitution | Can I substitute this subclass for its parent? | Ensure behavioral compatibility in inheritance |
| **I**SP | Interface Segregation | Are there unused methods in this interface? | Create focused, cohesive interfaces |
| **D**IP | Dependency Inversion | Do high-level modules depend on low-level details? | Inject dependencies, depend on abstractions |

## Quick Implementation Checklist

### Before Writing Code
- [ ] **Identify the single responsibility** - What is this class/module supposed to do?
- [ ] **Design for extension** - How might this need to change in the future?
- [ ] **Define interfaces first** - What contract should this fulfill?
- [ ] **Consider dependencies** - What does this need from other components?

### Code Review Questions
- [ ] **SRP**: Does each class have a single, clear responsibility?
- [ ] **OCP**: Can new behavior be added without modifying existing code?
- [ ] **LSP**: Are subclasses truly substitutable for their parents?
- [ ] **ISP**: Are interfaces focused and client-specific?
- [ ] **DIP**: Do high-level modules avoid depending on implementation details?

## Common Rust Patterns

### SRP - Separate Concerns
```rust
// ❌ Multiple responsibilities
struct UserManager { /* saves, emails, validates */ }

// ✅ Single responsibilities  
struct UserRepository { /* only saves */ }
struct EmailService { /* only emails */ }
struct UserValidator { /* only validates */ }
```

### OCP - Use Traits for Extension
```rust
// ✅ Extensible without modification
trait Shape {
    fn area(&self) -> f64;
}

struct Circle { radius: f64 }
impl Shape for Circle { /* implementation */ }
// New shapes just implement the trait
```

### LSP - Maintain Behavioral Contracts
```rust
// ✅ Both implement Shape consistently
trait Shape {
    fn area(&self) -> f64;
}
// Rectangle and Square both honor the Shape contract
```

### ISP - Small, Focused Traits
```rust
// ✅ Segregated interfaces
trait Workable { fn work(&self); }
trait Feedable { fn eat(&self); }
trait Restable { fn sleep(&self); }

// Types implement only what they need
impl Workable for Robot { /* only work */ }
impl Workable + Feedable + Restable for Human { /* all three */ }
```

### DIP - Inject Dependencies
```rust
// ✅ Depend on abstraction
trait Logger { fn log(&self, msg: &str); }

struct Service<T: Logger> {
    logger: T,  // Depends on trait, not concrete type
}
```

## Refactoring Signals

| **Anti-Pattern** | **SOLID Violation** | **Quick Fix** |
|------------------|---------------------|---------------|
| Large classes doing everything | SRP | Split by responsibility |
| Switch/match statements for types | OCP | Use traits/polymorphism |
| Subclasses throwing exceptions | LSP | Fix inheritance hierarchy |
| Empty method implementations | ISP | Segregate interfaces |
| Direct dependency instantiation | DIP | Constructor injection |

## Error Recovery Protocol

1. **Attempt 1**: Diagnose & apply targeted fix
2. **Attempt 2**: Try different approach if same error
3. **Attempt 3**: Broader rethink of assumptions
4. **After 3 failures**: Escalate with details

## When to Read the Full Guide

- **Learning**: First time implementing SOLID in Rust
- **Complex refactoring**: Large architectural changes
- **Code review**: Detailed pattern examples needed
- **Debugging**: Understanding why a design violates principles

---

**Remember**: SOLID principles guide good design but shouldn't be followed blindly. Use judgment to apply them appropriately to your specific context.