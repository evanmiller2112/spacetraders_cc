# Marketplace Trading System Fix Plan

## Overview
Complete overhaul of the marketplace trading system to fix electronics procurement and other manufactured goods purchasing for contract fulfillment.

## Critical Issues Identified

### 1. Budget Logic (contracts.rs:711-718)
- **Current**: `max_price_per_unit = budget / needed` - too conservative
- **Fix**: Use reasonable price thresholds (e.g., 5000 credits/unit max for electronics)
- **Priority**: HIGH

### 2. Hardcoded System References (contracts.rs:722,793)
- **Current**: Hardcoded to "X1-N5" system
- **Fix**: Dynamic system discovery based on ship locations
- **Priority**: HIGH

### 3. HIGH_TECH Waypoint Recognition
- **Current**: Only searches generic "MARKETPLACE" traits
- **Fix**: Prioritize HIGH_TECH waypoints for electronics/manufactured goods
- **Priority**: HIGH

### 4. Transaction Limit Handling (contracts.rs:924)
- **Current**: Single purchase attempt for all units
- **Fix**: Multiple transactions to handle API limits (e.g., 20 electronics per transaction)
- **Priority**: CRITICAL

### 5. Cargo Management Integration
- **Current**: No cargo space clearing before purchase
- **Fix**: Sell/jettison non-contract cargo before purchasing
- **Priority**: HIGH

### 6. Multi-Ship Cargo Distribution
- **Current**: Single ship capacity requirement
- **Fix**: Distribute large purchases across multiple ships
- **Priority**: MEDIUM

### 7. Supply Logic (contracts.rs:818-820)
- **Current**: Requires `trade_volume >= needed` (full amount)
- **Fix**: Accept partial availability with multiple transactions
- **Priority**: HIGH

### 8. Contract Delivery Integration
- **Current**: Trading ends at purchase
- **Fix**: Complete end-to-end: purchase → delivery → fulfillment
- **Priority**: MEDIUM

## Implementation Plan

### Phase 1: Core Trading Fixes (Critical Path)
1. **Fix Budget Logic**
   - Replace conservative `budget/needed` with reasonable per-unit limits
   - Add manufactured goods pricing database (electronics ~1500 credits)

2. **Add HIGH_TECH Recognition** 
   - Prioritize HIGH_TECH waypoints for manufactured goods
   - Add waypoint trait filtering by product type

3. **Implement Transaction Limits**
   - Add transaction limit database (electronics: 20 units/transaction)
   - Multiple purchase loop with error handling

4. **Dynamic System Discovery**
   - Remove hardcoded "X1-N5" references
   - Use ship locations for system targeting

### Phase 2: Cargo & Fleet Management
5. **Cargo Management Integration**
   - Pre-purchase cargo space clearing
   - Sell non-contract items at marketplaces
   - Jettison as fallback

6. **Multi-Ship Distribution**
   - Fleet cargo capacity calculation
   - Smart allocation across multiple ships
   - Parallel procurement operations

### Phase 3: End-to-End Integration
7. **Contract Delivery System**
   - Post-purchase navigation to delivery destination
   - Contract fulfillment integration
   - Success/failure reporting

8. **Comprehensive Testing**
   - Electronics procurement test
   - Large contract multi-ship test
   - Error handling validation

## Data Structures

### Enhanced Trading Plan
```rust
pub struct EnhancedTradingPlan {
    pub good: String,
    pub total_needed: i64,
    pub source_marketplace: String,
    pub price_per_unit: i64,
    pub transaction_limit: i32,
    pub ship_allocations: Vec<ShipAllocation>,
    pub delivery_destination: String,
    pub contract_id: String,
}

pub struct ShipAllocation {
    pub ship_symbol: String,
    pub units_to_purchase: i32,
    pub available_cargo_space: i32,
}
```

### Product Knowledge Database
```rust
pub struct ProductInfo {
    pub preferred_waypoint_traits: Vec<&'static str>,
    pub typical_price_range: (i64, i64),
    pub transaction_limit: Option<i32>,
    pub cargo_per_unit: i32,
}

// Database
const PRODUCT_DB: &[(&str, ProductInfo)] = &[
    ("ELECTRONICS", ProductInfo {
        preferred_waypoint_traits: vec!["HIGH_TECH", "MARKETPLACE"],
        typical_price_range: (1000, 2000),
        transaction_limit: Some(20),
        cargo_per_unit: 1,
    }),
    // ... other products
];
```

## Success Metrics

### Before Implementation
- ❌ Electronics procurement fails due to budget logic
- ❌ Single transaction attempts fail on API limits  
- ❌ No cargo management causes space issues
- ❌ Hardcoded system references limit flexibility

### After Implementation  
- ✅ Electronics successfully procured via HIGH_TECH waypoints
- ✅ Transaction limits handled with multiple purchases
- ✅ Cargo automatically managed before purchases
- ✅ Multi-ship coordination for large contracts
- ✅ End-to-end contract fulfillment

## Testing Strategy

### Unit Tests
- Transaction splitting logic
- Multi-ship allocation algorithm  
- Cargo management decision trees

### Integration Tests
- Full electronics procurement flow
- Large contract requiring multiple ships
- Error recovery scenarios

### Manual Validation
- Compare against successful manual process
- Verify API usage efficiency
- Confirm contract completion rates

## Risk Mitigation

### API Rate Limiting
- All requests go through centralized broker
- Conservative timing between operations
- Exponential backoff on failures

### Error Recovery
- Graceful handling of partial purchases
- Ship availability validation
- Market availability verification

### Resource Management
- Budget validation before operations
- Cargo space verification
- Fuel management for navigation

## Timeline
- Phase 1 (Critical): 2-3 hours implementation + testing
- Phase 2 (Fleet): 1-2 hours implementation + testing  
- Phase 3 (Integration): 1 hour implementation + testing
- **Total Estimated**: 4-6 hours for complete overhaul

## Priority Order
1. Transaction limit handling (CRITICAL - blocks all electronics)
2. HIGH_TECH waypoint recognition (HIGH - efficiency)
3. Budget logic fix (HIGH - prevents opportunities)
4. Cargo management (HIGH - prevents purchases)
5. Dynamic system discovery (MEDIUM - flexibility)
6. Multi-ship distribution (MEDIUM - scalability)
7. Contract delivery integration (LOW - completeness)