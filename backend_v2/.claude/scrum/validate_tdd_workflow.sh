#!/bin/bash
# TDD Workflow Validation Script
# Usage: ./validate_tdd_workflow.sh [BRANCH_NAME] [PR_NUMBER]

set -e

BRANCH=${1:-"HEAD"}
PR_NUMBER=${2:-"unknown"}

echo "🧪 TDD WORKFLOW VALIDATION"
echo "Branch: $BRANCH"
echo "PR: #$PR_NUMBER"
echo "=================================="

# Get commit history for the branch (excluding main)
echo ""
echo "📊 Commit History Analysis"
echo "--------------------------"
git log --oneline $BRANCH --not main

echo ""
echo "🔍 TDD Workflow Verification"
echo "-----------------------------"

# Check for TDD commit pattern
COMMITS=($(git log --format="%H" $BRANCH --not main))
COMMIT_COUNT=${#COMMITS[@]}

echo "Total commits: $COMMIT_COUNT"

if [ $COMMIT_COUNT -lt 2 ]; then
    echo "❌ INSUFFICIENT COMMITS: TDD requires at least 2 commits (test → implementation)"
    exit 1
fi

# Reverse array to get chronological order
for ((i=$COMMIT_COUNT-1; i>=0; i--)); do
    COMMIT=${COMMITS[i]}
    MESSAGE=$(git log --format="%s" -n 1 $COMMIT)
    FILES_CHANGED=$(git diff-tree --no-commit-id --name-only -r $COMMIT)

    echo ""
    echo "Commit $((COMMIT_COUNT-i)): $MESSAGE"
    echo "Files: $FILES_CHANGED"

    # Analyze commit for TDD patterns
    if [[ $MESSAGE =~ ^test.*TDD.*red ]]; then
        echo "✅ RED PHASE: Failing tests detected"
    elif [[ $MESSAGE =~ ^feat.*TDD.*green ]]; then
        echo "✅ GREEN PHASE: Implementation detected"
    elif [[ $MESSAGE =~ ^refactor.*TDD.*refactor ]]; then
        echo "✅ REFACTOR PHASE: Optimization detected"
    elif [[ $MESSAGE =~ ^test ]]; then
        echo "⚠️  TEST COMMIT: Check if this follows TDD pattern"
    elif [[ $MESSAGE =~ ^feat ]]; then
        echo "⚠️  IMPLEMENTATION: Check if tests were written first"
    else
        echo "❓ UNCLEAR: Commit message doesn't follow TDD pattern"
    fi
done

echo ""
echo "🔬 Test File Analysis"
echo "----------------------"

# Check for test files in the PR
TEST_FILES_ADDED=$(git diff --name-status main..$BRANCH | grep -E '^A.*test.*\.rs$' | wc -l)
TEST_FILES_MODIFIED=$(git diff --name-status main..$BRANCH | grep -E '^M.*test.*\.rs$' | wc -l)
IMPL_FILES_MODIFIED=$(git diff --name-status main..$BRANCH | grep -E '^[AM].*\.rs$' | grep -v test | wc -l)

echo "Test files added: $TEST_FILES_ADDED"
echo "Test files modified: $TEST_FILES_MODIFIED"
echo "Implementation files: $IMPL_FILES_MODIFIED"

if [ $((TEST_FILES_ADDED + TEST_FILES_MODIFIED)) -eq 0 ]; then
    echo "❌ NO TEST FILES: TDD requires test files"
    exit 1
fi

if [ $IMPL_FILES_MODIFIED -eq 0 ]; then
    echo "❌ NO IMPLEMENTATION: Changes appear to be test-only"
    exit 1
fi

echo ""
echo "🎯 TDD Best Practices Check"
echo "----------------------------"

# Check first commit for failing tests
FIRST_COMMIT=${COMMITS[$COMMIT_COUNT-1]}
if git show $FIRST_COMMIT --name-only | grep -q test; then
    echo "✅ GOOD: First commit includes test files"
else
    echo "❌ BAD: First commit should include test files"
fi

# Check for proper commit message patterns
RED_COMMITS=$(git log --format="%s" $BRANCH --not main | grep -c "red phase" || true)
GREEN_COMMITS=$(git log --format="%s" $BRANCH --not main | grep -c "green phase" || true)
REFACTOR_COMMITS=$(git log --format="%s" $BRANCH --not main | grep -c "refactor phase" || true)

echo ""
echo "TDD Phase Distribution:"
echo "- Red phase commits: $RED_COMMITS"
echo "- Green phase commits: $GREEN_COMMITS"
echo "- Refactor phase commits: $REFACTOR_COMMITS"

# Validate TDD workflow
TDD_SCORE=0

if [ $RED_COMMITS -gt 0 ]; then
    echo "✅ Has red phase commits"
    ((TDD_SCORE++))
else
    echo "❌ Missing red phase commits"
fi

if [ $GREEN_COMMITS -gt 0 ]; then
    echo "✅ Has green phase commits"
    ((TDD_SCORE++))
else
    echo "❌ Missing green phase commits"
fi

if [ $((TEST_FILES_ADDED + TEST_FILES_MODIFIED)) -gt 0 ]; then
    echo "✅ Includes test files"
    ((TDD_SCORE++))
else
    echo "❌ No test files"
fi

echo ""
echo "📊 TDD WORKFLOW SCORE: $TDD_SCORE/3"

if [ $TDD_SCORE -eq 3 ]; then
    echo ""
    echo "🎉 EXCELLENT TDD WORKFLOW!"
    echo "✅ All TDD requirements satisfied"
    echo "✅ Ready for code review"
    exit 0
elif [ $TDD_SCORE -ge 2 ]; then
    echo ""
    echo "⚠️  ACCEPTABLE TDD WORKFLOW"
    echo "✅ Most TDD requirements satisfied"
    echo "⚠️  Consider improving commit messages"
    exit 0
else
    echo ""
    echo "❌ POOR TDD WORKFLOW"
    echo "❌ TDD requirements not met"
    echo "❌ PR should be rejected for rework"
    echo ""
    echo "💡 TDD Requirements:"
    echo "1. Write failing tests first (red phase)"
    echo "2. Implement minimal code to pass (green phase)"
    echo "3. Refactor while keeping tests green (refactor phase)"
    echo "4. Use clear commit messages indicating TDD phases"
    exit 1
fi
