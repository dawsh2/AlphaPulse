# Torq Current Priorities - What Needs to be Done Tomorrow

*Updated: 2025-08-26 - Based on dynamic task scan*

## 🚨 EMERGENCY DATA INTEGRITY (HIGHEST PRIORITY)

### INTEGRITY-001: Fix Hardcoded Signal Data
- **File**: `services_v2/strategies/flash_arbitrage/src/signal_output.rs` (lines 159-256)
- **Problem**: Dashboard showing completely fake arbitrage data to users
- **Impact**: CRITICAL - System is lying to users about profits
- **Branch**: `git checkout -b integrity-001-fix-fake-data`
- **Must Fix**: Remove hardcoded `gas_cost_usd: 2.50`, `profit_usd: 150.0`, fake venues

### INTEGRITY-002: Remove Protocol-Violating DemoDeFiArbitrageTLV
- **Problem**: Type 255 TLV abuse bypassing protocol structure  
- **Impact**: CRITICAL - Protocol violations
- **Branch**: `git checkout -b integrity-002-remove-protocol-violations`

### SAFETY-001-NEW: Re-enable Profitability Guards
- **Problem**: Profitability validation disabled, potential financial losses
- **Impact**: HIGH - Prevents trading losses
- **Branch**: `git checkout -b safety-001-reenable-guards`

## ⚡ CRITICAL TASKS (This Week)

### EVENTS-001: Process All DEX Events (Not Just Swaps)
- **Problem**: Missing Mint, Burn, Sync events = incomplete market state
- **Impact**: MEDIUM - Incomplete state tracking
- **Files**: Pool event processing in adapters

### Repository Hygiene (CLEAN-001, CLEAN-002)
- **Status**: Both marked CRITICAL in task files
- **Impact**: Code quality and maintainability
- **Can be done in parallel with data integrity fixes**

## 🎯 IMMEDIATE ACTION PLAN FOR TOMORROW

### Priority 1 (EMERGENCY - Must Fix Before Any Production Use)
```bash
git checkout -b integrity-001-fix-fake-data
# Fix: services_v2/strategies/flash_arbitrage/src/signal_output.rs
# Remove all hardcoded values in send_arbitrage_analysis()
# Use real ArbitrageOpportunity data instead of fake values
```

### Priority 2 (CRITICAL Protocol Compliance)
```bash
git checkout -b integrity-002-remove-protocol-violations  
# Remove DemoDeFiArbitrageTLV and type 255 abuse
# Ensure all TLV messages follow proper protocol structure
```

### Priority 3 (Financial Safety)
```bash
git checkout -b safety-001-reenable-guards
# Re-enable profitability validation guards
# Ensure no trades execute without proper profit validation
```

## 🔧 TECHNICAL APPROACH

### For INTEGRITY-001:
1. **Identify Hardcoded Data**: All fake values in signal generation
2. **Use Real Data Sources**: Connect to actual ArbitrageOpportunity calculations
3. **Test with Real Market**: Verify signals show authentic arbitrage opportunities
4. **Validation**: No hardcoded numeric values remain

### For INTEGRITY-002:
1. **Find Protocol Violations**: Locate type 255 TLV usage
2. **Replace with Proper Types**: Use correct TLV types from 1-79 ranges
3. **Protocol Compliance**: Ensure all messages follow TLV structure
4. **Testing**: Verify protocol parsing works correctly

### For SAFETY-001-NEW:
1. **Find Disabled Guards**: Locate commented-out profitability checks
2. **Re-enable Validation**: Restore profit calculation validation
3. **Test Edge Cases**: Ensure unprofitable trades are blocked
4. **Financial Safety**: Prevent any potential losses

## 📊 SUCCESS METRICS

### Data Integrity Fixed When:
- [ ] Dashboard shows real arbitrage opportunities (not fake data)
- [ ] All profit calculations come from actual market analysis
- [ ] No hardcoded venues, tokens, or profit values
- [ ] Protocol compliance: no type 255 TLV abuse

### System Ready When:
- [ ] Real market data flowing end-to-end
- [ ] Profitability guards preventing losses
- [ ] All DEX events processed (Mint, Burn, Sync, Swap)
- [ ] Complete transparency in signal generation

## 🚫 WHAT NOT TO WORK ON TOMORROW

- **Mycelium Runtime** (MYCEL-001 through 007) - Future performance optimization
- **Protocol Optimization** (OPT-001 through 004) - Not production blocking  
- **MVP Tasks** (MVP-001, MVP-002) - Architecture improvements for later

## ⚠️ CRITICAL REMINDER

**The system currently shows fake data to users.** This is the highest priority issue that must be fixed immediately. Everything else is secondary until we have complete data integrity and transparency.

**Before any production deployment:** INTEGRITY-001 and INTEGRITY-002 must be resolved. The system cannot go live while showing fabricated arbitrage opportunities.