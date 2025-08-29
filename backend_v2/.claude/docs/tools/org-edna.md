# Org-Edna Dependency Management

Org-edna provides powerful task dependency management through TRIGGER and BLOCKER properties, replacing our simple `:DEPENDS:` system with automatic state transitions and bidirectional dependencies.

## Quick Start

```elisp
;; Add to your Emacs config
(require 'org-edna)
(org-edna-mode 1)
```

## Core Concepts

### BLOCKER Property
Prevents a task from being marked DONE until conditions are met:
```org
:BLOCKER: ids(BUILD-001-TESTS) todo?(DONE)
```

### TRIGGER Property  
Automatically updates other tasks when this one completes:
```org
:TRIGGER: ids(BUILD-002) todo!(NEXT)
```

## Common Patterns

### TDD (Test-Driven Development)
```org
** TODO Test Design for Feature           :testing:tdd:
   :PROPERTIES:
   :ID:          FEAT-001-TESTS
   :TRIGGER:     ids(FEAT-001) todo!(NEXT)
   :END:

** TODO Feature Implementation            :implementation:
   :PROPERTIES:
   :ID:          FEAT-001
   :BLOCKER:     ids(FEAT-001-TESTS) todo?(DONE)
   :END:
```
When tests complete → implementation automatically becomes NEXT!

### Sequential Tasks
```org
** TODO Step 1
   :PROPERTIES:
   :TRIGGER:     next-sibling todo!(NEXT)
   :END:

** TODO Step 2
   :PROPERTIES:
   :TRIGGER:     next-sibling todo!(NEXT)
   :END:

** TODO Step 3
   :PROPERTIES:
   :TRIGGER:     parent todo!(DONE)
   :END:
```
Each step automatically activates the next one.

### Parallel Tasks with Parent
```org
* TODO Project Goal
  :PROPERTIES:
  :BLOCKER:     children todo?(DONE)
  :TRIGGER:     children todo!(NEXT)
  :END:

** TODO Parallel Task 1
** TODO Parallel Task 2  
** TODO Parallel Task 3
```
Parent waits for all children, but children can run in parallel.

### Complex Dependencies
```org
:BLOCKER: ids(TASK-A TASK-B TASK-C) todo?(DONE)
```
Wait for multiple tasks to complete.

```org
:BLOCKER: ids(TASK-A) todo?(DONE) ids(TASK-B) todo?(|DONE|CANCELLED)
```
Different conditions for different dependencies.

## Edna Syntax

### Finders (What tasks to target)
- `ids(ID1 ID2 ...)` - Find tasks by ID property
- `siblings` - All sibling tasks at same level
- `children` - All child tasks
- `parent` - Parent task
- `previous-sibling` - Task immediately before
- `next-sibling` - Task immediately after
- `chain-find-next` - Next task in chain

### Conditions (When to block)
- `todo?(STATE)` - Has specific TODO state
- `todo?(|STATE1|STATE2)` - Has one of these states
- `!todo?(STATE)` - Does NOT have this state
- `has-property?("PROP")` - Has property set
- `match?("priority<\"B\"")` - Match expression

### Actions (What to do on trigger)
- `todo!(STATE)` - Change TODO state
- `scheduled!("++1d")` - Schedule task
- `set-property!("PROP" "value")` - Set property
- `set-priority!("A")` - Set priority

## Key Commands

```elisp
C-c C-x n   ; Find NEXT actionable tasks
C-c C-x d   ; Show dependencies for current task
C-c C-x p   ; Auto-promote TODO → NEXT
C-c C-x s   ; Find stuck projects
```

## Migration from Simple Dependencies

### Before (Simple `:DEPENDS:`)
```org
** TODO Implementation Task
   :PROPERTIES:
   :DEPENDS:     TEST-001
   :END:
```

### After (Org-Edna)
```org
** TODO Test Task
   :PROPERTIES:
   :ID:          TEST-001
   :TRIGGER:     ids(IMPL-001) todo!(NEXT)
   :END:

** TODO Implementation Task
   :PROPERTIES:
   :ID:          IMPL-001
   :BLOCKER:     ids(TEST-001) todo?(DONE)
   :END:
```

## Advantages Over Previous Systems

### vs. Simple `:DEPENDS:`
- **Automatic state changes** - No manual status updates
- **Bidirectional** - Test knows about implementation
- **Rich conditions** - Not just "is done"

### vs. YAML Frontmatter
- **Native org-mode** - No external Python scripts
- **No manual sync** - TRIGGER/BLOCKER automatically linked
- **Agenda integration** - Works with org agenda views
- **Dynamic finders** - siblings, children, etc.

## Debugging Dependencies

### Check if task is blocked:
```elisp
M-x org-edna-blocker-function RET
```

### View dependency graph:
```bash
# Generate graphviz visualization
./tools/org-deps-to-graphviz.py active.org > deps.dot
dot -Tpng deps.dot -o deps.png
```

### Find circular dependencies:
```elisp
;; In *scratch* buffer
(org-edna-find-cycles)
```

## Best Practices

1. **Always use IDs** - More reliable than headlines
2. **Test → Implementation** - Use TDD pattern
3. **Avoid deep chains** - Max 3-4 levels deep
4. **Document complex logic** - Add comments for complex blockers
5. **Use NEXT keyword** - Distinguishes actionable from waiting

## Common Issues

### Task not unblocking?
- Check ID spelling matches exactly
- Verify dependent task is actually DONE
- Look for typos in BLOCKER syntax

### Trigger not firing?
- Ensure org-edna-mode is enabled
- Check TRIGGER syntax is valid
- Verify target task ID exists

### Performance with many tasks?
- Use specific IDs instead of broad finders
- Avoid complex match expressions
- Consider splitting large files

## Integration with Claude Code

The migration maintains compatibility with our existing tooling:
- `org_tasks.sh` - Updated to handle NEXT state
- `org_task_manager.el` - Edna-aware functions added
- Task templates - Include TRIGGER/BLOCKER examples
- Git worktrees - Work seamlessly with dependencies

## Resources

- [Org-Edna Manual](https://www.nongnu.org/org-edna-el/)
- [Migration Script](../tools/migrate-to-edna.py)
- [Configuration](../tools/org-edna-setup.el)
- [Examples](../tasks/active.org)