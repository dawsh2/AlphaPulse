<!-- GENERATED FROM protocol/tlv_types.org - DO NOT EDIT DIRECTLY -->



# Overview

The Torq Protocol V2 uses Type-Length-Value (TLV) encoding for flexible, high-performance message passing across domain-specific relays. This document defines all TLV message types and their routing domains.

**Key Features**:

-   Zero-copy serialization for maximum performance
-   Domain-based routing for automatic message distribution
-   Bijective identifiers eliminate registry dependencies
-   Forward compatibility through graceful unknown type handling


# Performance Characteristics

<table border="2" cellspacing="0" cellpadding="6" rules="groups" frame="hsides">


<colgroup>
<col  class="org-left" />

<col  class="org-left" />

<col  class="org-left" />

<col  class="org-left" />
</colgroup>
<thead>
<tr>
<th scope="col" class="org-left">Domain</th>
<th scope="col" class="org-left">Processing Time</th>
<th scope="col" class="org-left">Throughput</th>
<th scope="col" class="org-left">Latency Target</th>
</tr>
</thead>
<tbody>
<tr>
<td class="org-left">Market Data</td>
<td class="org-left">&lt;35μs</td>
<td class="org-left">&gt;1M msg/sec</td>
<td class="org-left">Sub-millisecond</td>
</tr>

<tr>
<td class="org-left">Signals</td>
<td class="org-left">&lt;100μs</td>
<td class="org-left">&gt;100K msg/sec</td>
<td class="org-left">1-5ms</td>
</tr>

<tr>
<td class="org-left">Execution</td>
<td class="org-left">&lt;200μs</td>
<td class="org-left">&gt;50K msg/sec</td>
<td class="org-left">5-10ms</td>
</tr>
</tbody>
</table>


# Type Ranges by Domain


## Market Data Domain (Types 1-19)

High-frequency market events requiring minimal latency and maximum throughput.

-   **Trade (1)**: Executed trade with price, size, and timestamp
-   **Quote (2)**: Best bid/ask quotes from order books
-   **OrderBook (3)**: Full depth updates or snapshots
-   **ImbalanceIndicator (4)**: Pre-market or closing auction imbalances
-   **MarketStatus (5)**: Trading halts, circuit breakers, market state
-   **OptionGreeks (10)**: Delta, gamma, theta, vega for options
-   **ImpliedVolatility (11)**: IV surfaces and term structure


## Signal Domain (Types 20-39)

Trading signals and strategy outputs for portfolio and risk management.

-   **SignalIdentity (20)**: Signal metadata and routing information
-   **ArbitrageSignal (21)**: Cross-venue arbitrage opportunities
-   **DirectionalSignal (22)**: Trend and momentum indicators
-   **LiquiditySignal (23)**: Liquidity provision opportunities
-   **VolatilitySignal (24)**: Volatility regime changes


## Execution Domain (Types 40-79)

Order management and execution messages requiring guaranteed delivery.

-   **OrderNew (40)**: New order submission
-   **OrderCancel (41)**: Cancel existing order
-   **OrderModify (42)**: Modify order parameters
-   **OrderStatus (43)**: Execution status updates
-   **Fill (44)**: Partial or complete fill notifications
-   **Reject (45)**: Order rejection with reason codes


# Size Constraints


## Fixed Size Types

Zero validation overhead - size known at compile time:

-   Trade: 40 bytes
-   Quote: 48 bytes
-   SignalIdentity: 32 bytes


## Bounded Size Types

Single bounds check required:

-   SwapEvent: 60-200 bytes (variable addresses)
-   OrderStatus: 64-256 bytes (variable text)


## Variable Size Types

Dynamic allocation required - use sparingly in hot paths:

-   OrderBook: 100-64KB (full depth)
-   PositionSnapshot: 200-10KB (portfolio state)


# Implementation Status

<table border="2" cellspacing="0" cellpadding="6" rules="groups" frame="hsides">


<colgroup>
<col  class="org-left" />

<col  class="org-right" />

<col  class="org-left" />
</colgroup>
<thead>
<tr>
<th scope="col" class="org-left">Status</th>
<th scope="col" class="org-right">Count</th>
<th scope="col" class="org-left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td class="org-left">Production</td>
<td class="org-right">15</td>
<td class="org-left">Fully tested, deployed in production</td>
</tr>

<tr>
<td class="org-left">Beta</td>
<td class="org-right">8</td>
<td class="org-left">Implemented, undergoing validation</td>
</tr>

<tr>
<td class="org-left">Development</td>
<td class="org-right">5</td>
<td class="org-left">In active development</td>
</tr>

<tr>
<td class="org-left">Planned</td>
<td class="org-right">12</td>
<td class="org-left">Specified but not yet implemented</td>
</tr>
</tbody>
</table>


# Usage Guidelines


## Hot Path Optimization

For types in the Market Data domain:

-   Use fixed-size structures when possible
-   Avoid heap allocation
-   Implement zero-copy deserialization
-   Cache frequently accessed fields


## Message Construction

Always use `TLVMessageBuilder` for correct header and checksum:

    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, source);
    builder.add_tlv(TLVType::Trade, &trade_data);
    let message = builder.build();


## Type Discovery

Query types by domain for service-specific handling:

    let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
    for tlv_type in market_types {
        println!("{}: {}", tlv_type.type_number(), tlv_type.name());
    }

