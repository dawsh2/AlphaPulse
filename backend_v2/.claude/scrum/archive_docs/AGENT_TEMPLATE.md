# 🔒 MANDATORY AGENT INSTRUCTIONS - ENFORCEMENT TEMPLATE

## ⛔ CRITICAL: BRANCH ENFORCEMENT

**YOU ARE STRICTLY FORBIDDEN FROM WORKING ON THE MAIN BRANCH**

### MANDATORY FIRST COMMANDS (COPY AND RUN):
```bash
# CHECK 1: Verify you're NOT on main
git branch --show-current

# If the output is "main", you MUST run:
git checkout -b [YOUR-ASSIGNED-BRANCH]

# CHECK 2: Confirm you're on correct branch
git branch --show-current
# Output MUST show: [YOUR-ASSIGNED-BRANCH]
# If not, STOP and fix before proceeding
```

## 🚫 FORBIDDEN ACTIONS

You MUST NOT:
- ❌ Run `git checkout main` (except to create feature branch)
- ❌ Run `git merge` into main
- ❌ Run `git push origin main`
- ❌ Modify any branch other than your assigned branch
- ❌ Create additional branches beyond your assigned one
- ❌ Close or merge Pull Requests

## ✅ REQUIRED ACTIONS

You MUST:
- ✅ Work ONLY in branch: `[YOUR-ASSIGNED-BRANCH]`
- ✅ Commit ONLY to your feature branch
- ✅ Push ONLY your feature branch
- ✅ **PASS ALL TESTS** before creating PR
- ✅ **TEST WITH REAL DATA** where applicable
- ✅ Create a Pull Request with comprehensive test results
- ✅ Include performance benchmarks in PR description

## 🧪 MANDATORY TEST-FIRST DEVELOPMENT

### CRITICAL: Write Tests BEFORE Implementation

You MUST follow this exact sequence:

### Step 1: Write Failing Tests FIRST
```bash
# Create your tests based on acceptance criteria
cargo test --package [your-package] [new-test-name]
# MUST show: test result: FAILED (expected behavior)
```

### Step 2: Write Minimal Implementation
```bash
# Implement just enough to make tests pass
cargo test --package [your-package] [new-test-name]
# MUST show: test result: ok. 0 failed
```

### Step 3: Refactor & Optimize
```bash
# Improve implementation while keeping tests green
cargo test --package [your-package] [new-test-name]
# MUST remain: test result: ok. 0 failed
```

### Before Creating Any PR, You MUST:

1. **Unit Tests**: All tests you wrote FIRST must pass
```bash
cargo test --package [your-package] [test-name]
# MUST show: test result: ok. 0 failed
```

2. **Integration Tests**: Test with real system integration
```bash
cargo test --package [your-package] --test integration
# MUST pass all integration scenarios
```

3. **Performance Validation**: Ensure no regressions
```bash
cargo bench --package [your-package] [relevant-benchmark]
# MUST maintain performance targets
```

4. **Real Data Testing**: Use actual market data when applicable
```bash
# Example for pool/trading components:
RUST_LOG=debug cargo run --bin [component] -- --test-mode --duration=30s
# MUST process real data successfully
```

5. **End-to-End Validation**: Test complete workflow
```bash
# Example full pipeline test:
./scripts/test_component_e2e.sh [your-component]
# MUST complete without errors
```

### Real Data Testing Requirements

For **trading/market components**, you MUST test with:
- ✅ **Live exchange data** (limited duration: 30-60 seconds)
  - Crypto: Polygon, Binance, Coinbase, Kraken
  - TradFi: NYSE, NASDAQ, CME, ICE (via market data feeds)
  - DEX: Uniswap, SushiSwap, Curve (via RPC/WebSocket)
- ✅ **Real connection protocols**
  - WebSocket streams for real-time data
  - REST API endpoints for historical/reference data
  - RPC endpoints for blockchain interactions
  - Market data feeds (FIX, binary protocols)
- ✅ **Production message formats**
  - Native exchange formats (JSON, FIX, binary)
  - No synthetic/mocked message structures
  - Full message complexity including edge cases
- ✅ **Actual identifiers and addresses**
  - Real instrument symbols (BTCUSD, TSLA, etc.)
  - Live contract addresses for DeFi protocols
  - Valid venue/exchange identifiers

For **protocol/parsing components**, you MUST test with:
- ✅ **Real exchange message samples** from multiple venues
- ✅ **Historical data replays** covering various market conditions
- ✅ **Edge cases from production logs** (malformed, partial, delayed messages)
- ✅ **Performance under realistic load** (1000+ messages/second)
- ✅ **Cross-venue validation** ensuring format compatibility

### NO EXCEPTIONS POLICY

❌ **NEVER submit a PR with:**
- Failed unit tests
- Compilation warnings in production code
- Untested code paths
- Mock data only (for trading components)
- Performance regressions
- Missing integration validation

## 📋 VERIFICATION CHECKLIST

**CRITICAL: Git State is Shared Across All Terminals**

⚠️ **IMPORTANT**: When you switch branches, ALL terminal tabs in this project see the same branch. This means your changes affect everyone working in this repository immediately.

Before starting work:
```bash
# MANDATORY: Verify git state and create your branch
echo "=== GIT SAFETY CHECK ==="
CURRENT_BRANCH=$(git branch --show-current)
echo "Current branch (visible to ALL terminals): $CURRENT_BRANCH"

if [ "$CURRENT_BRANCH" = "main" ]; then
    echo "✅ Good: Starting from main branch"
    echo "➡️  Creating your feature branch NOW..."
    git checkout -b [YOUR-ASSIGNED-BRANCH]
    echo "✅ Switched to: $(git branch --show-current)"
    echo "🌍 NOTE: ALL terminals now show this branch"
elif [ "$CURRENT_BRANCH" = "[YOUR-ASSIGNED-BRANCH]" ]; then
    echo "✅ Perfect: Already on your assigned branch"
else
    echo "❌ ERROR: You are on wrong branch: $CURRENT_BRANCH"
    echo "❌ This affects ALL terminals!"
    echo "➡️  Switch to your assigned branch:"
    echo "   git checkout [YOUR-ASSIGNED-BRANCH]"
    exit 1
fi
```

## 🎯 YOUR TASK ASSIGNMENT

**Task ID**: [TASK-ID]
**Branch Name**: `[EXACT-BRANCH-NAME]`
**Task File**: `.claude/sprints/[SPRINT]/tasks/[TASK-FILE]`

### Task Execution Steps:
1. Read your complete task file
2. Verify you're on the correct branch (commands above)
3. Implement ONLY what's specified in the task
4. Commit to your branch with clear messages
5. Push your branch: `git push -u origin [YOUR-BRANCH]`
6. Report: "PR ready for review on branch [YOUR-BRANCH]"

## 🔄 COMMIT MESSAGE FORMAT

Use this format for ALL commits:
```
[type]([scope]): [description]

- [Detail 1]
- [Detail 2]

Task: [TASK-ID]
```

Types: feat, fix, test, docs, refactor, perf

## 📤 PULL REQUEST TEMPLATE

When creating your PR, use this MANDATORY format:
```markdown
## Task: [TASK-ID]
## Branch: [YOUR-BRANCH]

### Summary
[What you implemented]

### Changes
- [File 1]: [What changed]
- [File 2]: [What changed]

### 🧪 COMPREHENSIVE TESTING EVIDENCE (REQUIRED)

#### Test-First Development Evidence
```bash
# 1. FAILING TESTS (before implementation)
$ cargo test --package [package] [new-test-name]
[PASTE OUTPUT SHOWING INITIAL FAILURES - PROVES TESTS WRITTEN FIRST]

# 2. PASSING TESTS (after implementation)
$ cargo test --package [package] [new-test-name]
[PASTE OUTPUT SHOWING ALL TESTS NOW PASS - PROVES IMPLEMENTATION WORKS]
```

#### Unit Tests
```bash
$ cargo test --package [package] [test-name]
[PASTE FULL OUTPUT - MUST SHOW ALL PASSING]
```

#### Integration Tests
```bash
$ cargo test --package [package] --test integration
[PASTE FULL OUTPUT - MUST SHOW ALL PASSING]
```

#### Performance Benchmarks
```bash
$ cargo bench --package [package] [benchmark]
[PASTE RESULTS - MUST SHOW NO REGRESSION]
```

#### Real Data Testing
```bash
$ RUST_LOG=debug cargo run --bin [component] -- --test-mode --duration=60s
[PASTE OUTPUT SHOWING SUCCESSFUL PROCESSING OF REAL DATA]
```

#### End-to-End Validation
```bash
$ ./scripts/test_component_e2e.sh [component]
[PASTE FULL VALIDATION OUTPUT]
```

### 📊 Performance Impact
- Latency: [before] → [after]
- Memory: [before] → [after]
- Throughput: [before] → [after]

### 🔍 Real Data Validation
- **Exchange/Venue**: [Polygon/Binance/Kraken/NYSE/etc.]
- **Data type**: [WebSocket stream/REST API/RPC/Market data feed]
- **Duration tested**: [X seconds/minutes]
- **Messages processed**: [count]
- **Instruments tested**: [BTCUSD, ETH-USD, TSLA, pool addresses, etc.]
- **Success rate**: [%]
- **Latency observed**: [P50/P95/P99 milliseconds]
- **Error conditions tested**: [connection drops, malformed data, rate limits, etc.]
- **Cross-venue compatibility**: [if applicable]

### ✅ MANDATORY CHECKLIST
- [ ] Working in correct branch
- [ ] ALL unit tests passing (evidence above)
- [ ] ALL integration tests passing (evidence above)
- [ ] Performance benchmarks maintained (evidence above)
- [ ] Real data testing completed (evidence above)
- [ ] No compilation warnings in production code
- [ ] No commits to main
- [ ] End-to-end validation successful
- [ ] Ready for review
```

## ⚠️ SAFETY REMINDERS - SHARED GIT STATE

**CRITICAL**: Your git actions affect ALL terminals immediately!

1. **NEVER** type `git push origin main`
2. **NEVER** switch to main unless explicitly instructed
3. **ALWAYS** verify branch before commits: `git branch --show-current`
4. **REMEMBER**: When you switch branches, ALL terminals switch too
5. **IF UNSURE** ask: "Which branch should I be on?" before any git command
6. **AFTER TASK**: Create PR and report completion - don't switch branches
7. **NO EXCEPTIONS** to these rules - mistakes affect everyone

## 🚨 ERROR RECOVERY

If you accidentally commit to main:
```bash
# STOP IMMEDIATELY and report:
"ERROR: I may have committed to main.
Current branch: $(git branch --show-current)
Last commit: $(git log -1 --oneline)"

# Wait for instructions to fix
```

## 📊 COMPLIANCE TRACKING

Your compliance will be verified:
- Branch name matches assignment: ✓/✗
- Zero commits to main: ✓/✗
- PR created from correct branch: ✓/✗
- All work in assigned branch: ✓/✗

---

**FINAL REMINDER**: You are working on branch `[YOUR-BRANCH]`, NOT main.
Any commits to main will be rejected and must be redone.

**ACKNOWLEDGE**: Type "I confirm I will work only in branch [YOUR-BRANCH]" before starting.
