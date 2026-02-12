# Kowalski AI Agent Documentation

> **READ THIS FIRST**: This file serves as the single source of truth for any AI agent (Claude, Gemini, Cursor, etc.) working on the Pake repository. It aggregates architectural context, development workflows, and behavioral guidelines.

## 1. Philosophy & Guidelines

### Core Philosophy

- **Incremental progress over big bangs**: Break complex tasks into manageable stages.
- **Learn from existing code**: Understand patterns before implementing new features.
- **Clear intent over clever code**: Prioritize readability and maintainability.
- **Simple over complex**: Keep all implementations simple and straightforward - prioritize solving problems and ease of maintenance over complex solutions.

### Eight Honors and Eight Shames

- **Shame** in guessing APIs, **Honor** in careful research.
- **Shame** in vague execution, **Honor** in seeking confirmation.
- **Shame** in assuming business logic, **Honor** in human verification.
- **Shame** in creating interfaces, **Honor** in reusing existing ones.
- **Shame** in skipping validation, **Honor** in proactive testing.
- **Shame** in breaking architecture, **Honor** in following specifications.
- **Shame** in pretending to understand, **Honor** in honest ignorance.
- **Shame** in blind modification, **Honor** in careful refactoring.

### Quality Standards

- **English Only**: usage of any other language for comments is strictly forbidden.
- **No Unnecessary Comments**: For simple, obvious code, let the code speak for itself.
- **Self-Documenting Code**: Prefer explicit types and clear naming over inline documentation.
- **Composition over Inheritance**: Favor functional patterns where applicable (Rust).

## 2. Project Identity

**Name**: Kowalski
**Purpose**: tbd
**Core Value**: tbd
**Mechanism**: tbd

## 3. Technology Stack

- **Core Framework**: tbd
- **CLI**: tbd
- **Frontend**: tbd
- **Package Manager**: tbd

## 4. Repository Architecture

### Directory Structure

tbd

each subdirectory. package contains AGENT.md - which gets summary of current state of the package

## 5. Key Workflows

### Development

1. **Understand**: Study existing patterns in codebase.
2. **Plan**: Break complex work into stages.
3. **Test**: Write tests first (when applicable).
4. **Implement**: Minimal working solution.
5. **Refactor**: Optimize and clean up.


**Commands**:

tbd

### Building

tbd

### Release

tbd



## Critical Rules

### 1. Create Plan First
Never start a complex task without `task.md`. Non-negotiable.
Initial templase is in `task_template.md` file.

### Tracking changes
Always look into task.md file to understand what needs to be done. If there is no task.md file, create one. If there is a task.md file, update it with the current state of the project.


### 2. The 2-Action Rule
> "After every 2 view/browser/search operations, IMMEDIATELY save key findings to text files."

This prevents visual/multimodal information from being lost.

### 3. Read Before Decide
Before major decisions, read the plan file. This keeps goals in your attention window.

### 4. Update After Act
After completing any phase:
- Mark phase status: `in_progress` → `complete`
- Log any errors encountered
- Note files created/modified

### 5. Log ALL Errors
Every error goes in the task plan file. This builds knowledge and prevents repetition.

```markdown
## Errors Encountered
| Error | Attempt | Resolution |
|-------|---------|------------|
| FileNotFoundError | 1 | Created default config |
| API timeout | 2 | Added retry logic |
```

### 6. Never Repeat Failures
```
if action_failed:
    next_action != same_action
```
Track what you tried. Mutate the approach.

## The 3-Strike Error Protocol

```
ATTEMPT 1: Diagnose & Fix
  → Read error carefully
  → Identify root cause
  → Apply targeted fix

ATTEMPT 2: Alternative Approach
  → Same error? Try different method
  → Different tool? Different library?
  → NEVER repeat exact same failing action

ATTEMPT 3: Broader Rethink
  → Question assumptions
  → Search for solutions
  → Consider updating the plan

AFTER 3 FAILURES: Escalate to User
  → Explain what you tried
  → Share the specific error
  → Ask for guidance
```

## Read vs Write Decision Matrix

| Situation | Action | Reason |
|-----------|--------|--------|
| Just wrote a file | DON'T read | Content still in context |
| Viewed image/PDF | Write findings NOW | Multimodal → text before lost |
| Browser returned data | Write to file | Screenshots don't persist |
| Starting new phase | Read plan/findings | Re-orient if context stale |
| Error occurred | Read relevant file | Need current state to fix |
| Resuming after gap | Read all planning files | Recover state |

## The 5-Question Reboot Test

If you can answer these, your context management is solid:

| Question | Answer Source |
|----------|---------------|
| Where am I? | Current phase in task_plan.md |
| Where am I going? | Remaining phases |
| What's the goal? | Goal statement in plan |
| What have I learned? | findings.md |
| What have I done? | progress.md |

## When to Use This Pattern

**Use for:**
- Multi-step tasks (3+ steps)
- Research tasks
- Building/creating projects
- Tasks spanning many tool calls
- Anything requiring organization

**Skip for:**
- Simple questions
- Single-file edits
- Quick lookups


## Anti-Patterns

| Don't | Do Instead |
|-------|------------|
| Use TodoWrite for persistence | Create plan in task.md file |
| State goals once and forget | Re-read plan before decisions |
| Hide errors and retry silently | Log errors to plan file |
| Stuff everything in context | Store large content in files |
| Start executing immediately | Create plan file FIRST |
| Repeat failed actions | Track attempts, mutate approach |
| Create files in skill directory | Create files in your project |






## 6. Implementation Details

tbd

## 7. Common AI Tasks

tbd