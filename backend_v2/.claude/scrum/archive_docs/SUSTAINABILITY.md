# Sprint System Sustainability Guide

## 🎯 Vision
A self-maintaining, cruft-resistant sprint management system that scales from 1 to 10 developers without decay.

## 🏗️ System Architecture

### Core Components
```
.claude/scrum/                    # System core
├── templates/                    # Standardized formats
│   ├── SPRINT_PLAN.md           
│   ├── TASK_TEMPLATE.md         
│   └── TEST_RESULTS.md          
├── create-sprint.sh             # Sprint creator
├── task-manager.sh              # Status tracker
├── maintenance.sh               # Health checker
└── STANDARDIZATION.md           # Format specs

.claude/tasks/                   # Active work
├── sprint-XXX-name/             # Current sprints
│   ├── SPRINT_PLAN.md          
│   ├── TASK-*.md               
│   └── TEST_RESULTS.md         
└── archive/                     # Completed sprints
    └── sprint-XXX-name/         
```

### Automation Pipeline
```mermaid
graph LR
    A[Sprint Created] --> B[Tasks In Progress]
    B --> C[Tests Pass]
    C --> D[PR Merged]
    D --> E[Auto-Archive]
    E --> F[Metrics Updated]
```

## 🛡️ Sustainability Mechanisms

### 1. Standardization (Prevention)
- **Templates**: Enforce consistent format from day 1
- **YAML Frontmatter**: Machine-readable status tracking
- **Self-Contained Tasks**: Instructions embedded in each task
- **Branch Safety**: Git verification in every template

### 2. Automation (Efficiency)
- **create-sprint.sh**: Generate correct structure automatically
- **task-manager.sh**: Parse and track without manual updates
- **Auto-archive**: Move completed work out of sight
- **Git hooks**: Trigger archiving on PR merge

### 3. Maintenance (Health)
- **maintenance.sh**: Weekly health checks
- **Stale detection**: Find abandoned tasks/sprints
- **Format validation**: Catch drift early
- **Metrics tracking**: Monitor system health score

### 4. Three-Gate Verification (Quality)
- **Gate 1**: All tasks COMPLETE
- **Gate 2**: Tests documented passing
- **Gate 3**: PR merged to main
- **Result**: Only truly done work gets archived

## 📊 Key Metrics to Track

### Weekly Metrics (Automated)
```bash
# Run maintenance script
./.claude/scrum/maintenance.sh

# Provides:
- Health Score (target: >80/100)
- Format Compliance % 
- Stale Task Count
- Abandoned Sprint Count
```

### Monthly Metrics (Manual Review)
- **Sprint Velocity**: Tasks completed per sprint
- **Cycle Time**: Average task duration
- **Completion Rate**: Started vs finished tasks
- **Technical Debt**: Blocked/abandoned work

### Quarterly Review
- **Template Evolution**: Update based on learnings
- **Process Refinement**: Adjust sprint duration/size
- **Tool Updates**: Enhance scripts as needed

## 🚨 Early Warning Signs

### Yellow Flags (Monitor)
- Health score drops below 90
- Format compliance below 95%
- Any sprint > 7 days old
- More than 2 BLOCKED tasks

### Red Flags (Immediate Action)
- Health score below 80
- Direct commits to main
- Sprint with 20+ tasks
- Multiple IN_PROGRESS sprints per dev
- TEST_RESULTS.md consistently missing

## 🔄 Maintenance Schedule

### Daily (Developers)
```bash
# Check your sprint status
./.claude/tasks/sprint-XXX/check-status.sh

# Update task status when changing
vim TASK-001.md  # Update status field
```

### Weekly (Scrum Leader)
```bash
# Run maintenance
./.claude/scrum/maintenance.sh

# Archive completed sprints
./.claude/scrum/task-manager.sh auto-archive

# Clean merged branches
git branch --merged main | grep -v main | xargs -r git branch -d
```

### Monthly (Team)
- Review sprint velocity
- Update roadmap priorities
- Refine estimation accuracy
- Document lessons learned

### Quarterly (System Admin)
```bash
# Deep clean
find .claude/tasks/archive -mtime +90 -type d | tar -czf archive_Q1.tar.gz
rm -rf .claude/tasks/archive/sprint-*-very-old

# Update templates
vim .claude/scrum/templates/*.md

# Refactor tools if needed
vim .claude/scrum/task-manager.sh
```

## 💡 Best Practices for Longevity

### DO ✅
1. **Use templates religiously** - Never create tasks manually
2. **Update status immediately** - As soon as state changes
3. **Archive aggressively** - Completed = archived
4. **Run maintenance weekly** - Catch issues early
5. **Document test results** - No TEST_RESULTS.md = not done
6. **Keep sprints small** - 3-5 tasks, 5 days max
7. **One sprint at a time** - Finish before starting next

### DON'T ❌
1. **Skip format standards** - Breaks automation
2. **Leave tasks IN_PROGRESS** - Update or mark BLOCKED
3. **Create mega-sprints** - 20+ tasks = guaranteed failure
4. **Ignore health warnings** - Address immediately
5. **Bypass three gates** - All must pass for completion
6. **Work on main** - Always use feature branches
7. **Delay archiving** - Cruft accumulates fast

## 🔧 Troubleshooting

### Problem: Format drift
```bash
# Fix: Reformat existing tasks
for file in .claude/tasks/sprint-*/TASK-*.md; do
  # Add YAML frontmatter if missing
  if ! grep -q "^---" "$file"; then
    # Add frontmatter based on existing format
  fi
done
```

### Problem: Abandoned sprints
```bash
# Fix: Force archive or delete
./.claude/scrum/task-manager.sh archive-sprint sprint-XXX --force
# OR
rm -rf .claude/tasks/sprint-XXX-abandoned
```

### Problem: Stale branches everywhere
```bash
# Fix: Aggressive cleanup
git branch | grep -v main | xargs git branch -D  # Nuclear option
```

## 📈 Scaling Strategy

### 1-2 Developers (Current)
- Manual status updates work fine
- Weekly maintenance sufficient
- Single active sprint

### 3-5 Developers
- Add assignee tracking to templates
- Implement dependency management
- Multiple parallel sprints OK
- Consider daily standups

### 5-10 Developers
- Integrate with external tools (Jira/Linear)
- Add automated status detection from git
- Implement velocity tracking
- Consider dedicated scrum master

### 10+ Developers
- Move to professional tool
- Keep templates as documentation
- Maintain three-gate philosophy
- Archive this system (it served well!)

## 🎓 Onboarding New Developers

### Day 1 Checklist
- [ ] Read STANDARDIZATION.md
- [ ] Run create-sprint.sh to see structure
- [ ] Review recent archived sprint
- [ ] Understand three-gate system
- [ ] Practice with test task

### First Sprint Rules
1. Use templates exactly as provided
2. Never work on main branch
3. Update status immediately
4. Create TEST_RESULTS.md
5. Wait for PR approval

## 🏆 Success Criteria

The system is sustainable when:
- **Health Score**: Consistently >85/100
- **Format Compliance**: >95% of tasks
- **Archive Rate**: 100% of completed sprints
- **Stale Tasks**: <5% at any time
- **Sprint Velocity**: Predictable ±20%
- **Developer Satisfaction**: "It just works"

## 🔮 Future Evolution

### Planned Enhancements
1. **Git-based status detection**: Auto-update from commits
2. **Velocity charts**: Automated burndown graphs
3. **Slack integration**: Status notifications
4. **AI summarization**: Sprint retrospectives

### Preserve Core Principles
Whatever evolves, maintain:
- File-based transparency
- Git-native workflow
- Three-gate verification
- Template standardization
- Aggressive archiving
- Weekly maintenance

## 📚 Reference Documents
- `STANDARDIZATION.md` - Format specifications
- `TEMPLATES.md` - Template documentation
- `ARCHIVING.md` - Archive process
- `scrum-leader.md` - Agent instructions

## Final Word
This system is designed to be **boring and reliable**. It should fade into the background, quietly organizing work while developers focus on code. If you find yourself fighting the system, something is wrong - either fix the templates or run maintenance. The system should feel like it maintains itself.

**Remember**: A sustainable system is one that gets better with use, not worse. Every sprint should be slightly smoother than the last.