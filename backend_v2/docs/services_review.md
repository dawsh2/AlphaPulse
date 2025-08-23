# Services.md Review - Inconsistencies Found

## Major Inconsistency: Execution Engine for Flash Loan Arbitrage

### Current State (INCORRECT)
Services.md describes:
- ArbitrageStrategy → Signal Relay → **Execution Engine** → Smart Contract
- Execution Engine receives ArbitrageSignal and builds transactions
- Sections 335-450 describe Execution Engine handling flash loan arbitrage

### Should Be (CORRECT)
Per our architecture clarification:
- ArbitrageStrategy is **self-contained**
- Market Data → ArbitrageStrategy → **Direct to Blockchain**
- Strategy detects AND executes atomically
- No Execution Engine involvement for flash loans

## Sections Needing Updates

### 1. Lines 335-450: "Execution Engine Architecture"
- Currently describes ExecutionEngine processing ArbitrageSignal
- Should clarify ExecutionEngine is for risk-managed strategies only
- Flash loan strategies execute independently

### 2. Lines 198-320: "Arbitrage Strategy Architecture"  
- Should emphasize the strategy is self-contained
- Includes its own transaction builder and submission
- Not just detection but also execution

### 3. Lines 986-989: Service List
- Currently lists "Execution Engine" as part of flash loan flow
- Should clarify execution is within ArbitrageStrategy itself

### 4. Lines 90-98: Signal Generation Layer
- Currently implies signals go to "execution systems"
- Should clarify flash loan strategies execute internally

## Other Observations

### Good Aspects
- Protocol.md is correctly updated
- Clear about flash loans and atomic execution
- Venue abstraction is well designed

### Minor Issues
- Some references to "Risk Monitor" for flash loans (should be post-trade only)
- Signal Relay role unclear for self-contained strategies

## Recommendation

Services.md needs significant updates to reflect that:
1. Flash loan arbitrage strategies are self-contained
2. Execution Engine is only for risk-managed strategies
3. ArbitrageStrategy includes detection AND execution
4. Signal Relay might not be needed for flash loan strategies

This is a fundamental architectural point that needs to be consistent across all documentation.