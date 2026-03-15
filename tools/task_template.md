# Task Plan: [Brief Description]
<!-- 
  WHAT: This is your roadmap for the entire task. Think of it as your "working memory on disk."
  WHY: After 50+ tool calls, your original goals can get forgotten. This file keeps them fresh.
  WHEN: Create this FIRST, before starting any work. Update after each phase completes.
  
  SOLID PRINCIPLES REMINDER:
  - Single Responsibility: Each class/module should have one reason to change
  - Open/Closed: Open for extension, closed for modification
  - Liskov Substitution: Subclasses must be substitutable for their parents
  - Interface Segregation: Clients shouldn't depend on unused interfaces
  - Dependency Inversion: Depend on abstractions, not concretions
-->

## Goal
<!-- 
  WHAT: One clear sentence describing what you're trying to achieve.
  WHY: This is your north star. Re-reading this keeps you focused on the end state.
  EXAMPLE: "Create a Python CLI todo app with add, list, and delete functionality."
-->
[One sentence describing the end state]

## Current Phase
<!-- 
  WHAT: Which phase you're currently working on (e.g., "Phase 1", "Phase 3").
  WHY: Quick reference for where you are in the task. Update this as you progress.
-->
Phase 1

## Phases
<!-- 
  WHAT: Break your task into 3-7 logical phases. Each phase should be completable.
  WHY: Breaking work into phases prevents overwhelm and makes progress visible.
  WHEN: Update status after completing each phase: pending → in_progress → complete
-->

### Phase 1: Requirements & Discovery
<!-- 
  WHAT: Understand what needs to be done and gather initial information.
  WHY: Starting without understanding leads to wasted effort. This phase prevents that.
-->
- [ ] Understand user intent and business requirements
- [ ] Identify constraints, dependencies, and integration points
- [ ] Review existing codebase and architectural patterns
- [ ] Read component-specific AGENT.md files if applicable
- [ ] Document findings and architectural context
- **Status:** in_progress
<!-- 
  STATUS VALUES:
  - pending: Not started yet
  - in_progress: Currently working on this
  - complete: Finished this phase
-->

### Phase 2: Planning & Architecture
<!-- 
  WHAT: Decide how you'll approach the problem and what structure you'll use.
  WHY: Good planning prevents rework. Document decisions so you remember why you chose them.
-->
- [ ] Define technical approach following SOLID principles
- [ ] Design interfaces and abstractions first (DIP)
- [ ] Plan for extensibility without modification (OCP)
- [ ] Ensure single responsibility for each component (SRP)
- [ ] Create focused, client-specific interfaces (ISP)
- [ ] Design inheritance hierarchies carefully (LSP)
- [ ] Document architectural decisions with rationale
- **Status:** pending

### Phase 3: Implementation
<!-- 
  WHAT: Actually build/create/write the solution.
  WHY: This is where the work happens. Break into smaller sub-tasks if needed.
-->
- [ ] Implement interfaces and abstractions first
- [ ] Create concrete implementations following SRP
- [ ] Use dependency injection for loose coupling (DIP)
- [ ] Write small, focused methods and classes
- [ ] Follow established coding standards and patterns
- [ ] Write code to files before executing
- [ ] Test incrementally with unit tests
- **Status:** pending

### Phase 4: Testing & Verification
<!-- 
  WHAT: Verify everything works and meets requirements.
  WHY: Catching issues early saves time. Document test results for future reference.
-->
- [ ] Verify all functional requirements met
- [ ] Test SOLID principles compliance:
  - [ ] Single Responsibility: Each class has one clear purpose
  - [ ] Open/Closed: Can extend without modifying existing code
  - [ ] Liskov Substitution: Subclasses work as drop-in replacements
  - [ ] Interface Segregation: No unused interface methods
  - [ ] Dependency Inversion: High-level modules independent of low-level details
- [ ] Run unit tests and integration tests
- [ ] Test error handling and edge cases
- [ ] Document test results and coverage
- [ ] Fix any issues found following 3-strike protocol
- **Status:** pending

### Phase 5: Delivery
<!-- 
  WHAT: Final review and handoff to user.
  WHY: Ensures nothing is forgotten and deliverables are complete.
-->
- [ ] Review all output files
- [ ] Ensure deliverables are complete
- [ ] Deliver to user
- **Status:** pending

## Key Questions
<!-- 
  WHAT: Important questions you need to answer during the task.
  WHY: These guide your research and decision-making. Answer them as you go.
  EXAMPLE: 
    1. Should tasks persist between sessions? (Yes - need file storage)
    2. What format for storing tasks? (JSON file)
    3. How can we ensure this design is extensible? (Use interfaces and dependency injection)
-->
1. [Question to answer]
2. [Question to answer]
3. [SOLID-related question if applicable]

## Decisions Made
<!-- 
  WHAT: Technical and design decisions you've made, with the reasoning behind them.
  WHY: You'll forget why you made choices. This table helps you remember and justify decisions.
  WHEN: Update whenever you make a significant choice (technology, approach, structure).
  EXAMPLE:
    | Use Repository pattern for data access | Follows SRP and DIP, makes testing easier |
    | Create separate interfaces for read/write | Follows ISP, clients only depend on what they use |
-->
| Decision | Rationale | SOLID Principle(s) |
|----------|-----------|-------------------|
|          |           |                   |

## Errors Encountered
<!-- 
  WHAT: Every error you encounter, what attempt number it was, and how you resolved it.
  WHY: Logging errors prevents repeating the same mistakes. This is critical for learning.
  WHEN: Add immediately when an error occurs, even if you fix it quickly.
  EXAMPLE:
    | FileNotFoundError | 1 | Check if file exists, create empty list if not | Added proper error handling |
    | Tight coupling issue | 2 | Introduced interface to decouple components | Applied DIP principle |
-->
| Error | Attempt | Resolution | Lessons Learned |
|-------|---------|------------|----------------|
|       | 1       |            |                |

## Architecture & Design Notes
<!-- 
  WHAT: Important architectural insights and design patterns discovered during implementation
  WHY: Capture design knowledge for future reference and team learning
-->
- Design patterns used: [e.g., Repository, Factory, Strategy]
- SOLID principle applications: [specific examples from your implementation]
- Integration points: [how this connects with existing systems]
- Future extensibility: [planned extension points]

## Development Notes
<!-- 
  REMINDERS:
  - Update phase status as you progress: pending → in_progress → complete
  - Re-read this plan before major decisions (attention manipulation)
  - Log ALL errors - they help avoid repetition
  - Never repeat a failed action - mutate your approach instead
  - Always consider SOLID principles when making design decisions
  - Read component AGENT.md files before modifying existing code
-->
- Update phase status as you progress: pending → in_progress → complete
- Re-read this plan before major decisions (attention manipulation)
- Log ALL errors - they help avoid repetition
