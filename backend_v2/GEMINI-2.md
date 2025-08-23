# GEMINI-2: Architectural Deep Dive: Relays and State Management

This document provides the detailed reasoning behind the architectural decisions for the `relays` and the new `libs/state` library, as outlined in `GEMINI-1.md`.

## 1. The Role of the `relays` Directory

The `relays` are a distinct architectural tier that sits between the `infra` (the message bus) and `services_v2` (the application logic). This placement resolves a key ambiguity: relays are services, but they are a special class of foundational service that acts as the application's central nervous system.

### Trade-Offs and Decision

1.  **Why not in `infra/`?**
    *   The `infra` layer is application-agnostic. It moves bytes.
    *   A relay, however, is application-aware. It must parse the `MessageHeader` to read the `relay_domain` field. This is a piece of business logic, even if it's a simple one. Placing this logic in `infra` would violate the principle of separation of concerns.

2.  **Why not in `services_v2/`?**
    *   `services_v2` contains the business logic endpoints (actors) of the system: strategies, execution engines, etc.
    *   Relays are not endpoints; they are the routers *between* endpoints. Lumping them in with general services would obscure their critical role as the primary routing fabric.

3.  **Conclusion: A Dedicated Tier**
    *   Placing relays in their own top-level directory (`relays/`) correctly identifies them as a unique architectural layer. They are services, but they are part of the core routing infrastructure, distinct from both the generic bus and the specific business logic.

### Design Principle: Configuration over Code

A crucial design principle for the relay tier is that the core routing logic should be generic and reusable.

- **The `Relay` Core**: There should be a single, generic `Relay` implementation. Its logic is simple:
    1.  Connect to the `infra` message bus.
    2.  Receive a message.
    3.  Parse the `MessageHeader`.
    4.  Check if the `relay_domain` matches its configured domain.
    5.  If it matches, publish the message to its designated output channel on the bus.
    6.  If not, ignore the message.

- **Configuration**: The specific relays (`MarketDataRelay`, `SignalRelay`) are instances of this generic relay, differentiated only by their configuration. A `market_data.toml` might specify `domain = 1`, while a `signal.toml` would specify `domain = 2`.

This approach is highly reusable. To build a completely different application (e.g., a chat server), one could reuse the entire `infra` and `relays` code. The only changes required would be to write a new protocol library (defining new TLVs), new `services_v2` (the chat clients and servers), and new relay configuration files.

## 2. The `libs/state` Architecture: A "Core + Libraries" Workspace Model

The management of state (e.g., order books, DEX pools, indicators) presents a critical architectural challenge: we must avoid both a single, tightly-coupled monolithic library and the massive code duplication that would come from each service implementing its own state logic from scratch.

The solution is a hybrid "workspace" model that abstracts the shared, foundational logic into a core library, which is then used by smaller, domain-specific state libraries, all organized under a single `libs/state` directory.

### The "Core + Libraries" Design

1.  **`libs/state/core` - The Foundation**:
    *   A foundational library is created at `libs/state/core`.
    *   **Purpose**: This crate contains *only* the generic, application-agnostic logic required for any kind of state management from an event stream.
    *   **Contents**:
        *   Core traits like `Stateful` or `EventProcessor`.
        *   Logic for handling message sequence gaps.
        *   Mechanisms for creating and applying state snapshots for recovery.
        *   The generic, reusable message processing loop.
    *   This library is the single source of truth for *how* state is managed.

2.  **Domain-Specific Libraries - The Implementations**:
    *   For each logical domain, we create a separate, focused library within the `libs/state` workspace that **depends on the `core` library**.
    *   **Examples**:
        *   `libs/state/market`: Implements the `Stateful` trait for `OrderBook` and `Pool`. It contains the specific business logic for how a `TradeTLV` updates an order book.
        *   `libs/state/execution`: Implements the `Stateful` trait for `Order`. It knows how to process a `FillTLV` to update an order's status.
        *   `libs/state/portfolio`: Implements state logic for `Position`.
    *   **Purpose**: These libraries contain the specific business rules for their domain, while delegating all the complex machinery of event processing to the `core` library.

### Directory Structure & Dependencies

This model results in a clean and scalable workspace structure within a single `libs/state/` directory:

```
libs/
└── state/
    ├── Cargo.toml        # The workspace definition file
    ├── core/             # The `state-core` crate (generic foundation)
    ├── market/           # The `state-market` crate (pools, order books)
    ├── execution/        # The `state-execution` crate (order state)
    └── portfolio/        # The `state-portfolio` crate (position state)
```

### Benefits of This Model

This architecture directly resolves the architectural dilemma and provides numerous benefits:

*   **DRY (Don't Repeat Yourself)**: The complex, foundational state logic is written once in `state/core` and never duplicated.
*   **High Cohesion**: Logic for market state is completely isolated in its own library, separate from execution or portfolio logic.
*   **Fine-Grained Dependencies**: Services can depend only on the state they need. A `strategy` can use `state/market` without pulling in any dependencies from `state/execution`.
*   **Clear Ownership**: The `execution` team can own the `state/execution` library, while the `strategies` team owns `state/market`, promoting team autonomy.
*   **Supports the Dual-Model**: This structure continues to support both deployment models. A live-trading `strategy` can embed `state/market` for low latency, while a backtesting `StateService` can embed `state/market`, `state/execution`, and `state/portfolio` to provide a holistic, shared view of the world.