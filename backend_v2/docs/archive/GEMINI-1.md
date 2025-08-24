# GEMINI-1: Core Application Architecture Refactoring

## 1. Context and Motivation

This document outlines a plan to refactor the core application architecture within `backend_v2`. The goal is to establish a clear, logical, and scalable structure for our services, libraries, and infrastructure code.

The current structure has ambiguities in the roles of different components, particularly the message `relays`. This plan clarifies these roles by organizing the application code into distinct architectural tiers.

**Note:** This refactoring *only* concerns the core application source code. Existing top-level directories such as `protocol_v2`, `tests`, `config`, `docs`, and `scripts` are outside the scope of this plan and will remain unchanged.

## 2. The Three-Tier Application Architecture

The core application code will be organized into three new top-level directories, clarifying the separation of concerns:

1.  **`infra/`**: The foundational, application-agnostic transport layer. This is the message bus implementation (e.g., shared memory, sockets).
2.  **`relays/`**: The application's routing layer. These are special services that sit on the `infra` bus and direct messages based on their domain (`market_data`, `signal`, etc.).
3.  **`services_v2/`**: The application's business logic. These are the endpoints (actors) that produce and consume messages to perform their tasks (e.g., `adapters`, `strategies`).

This three-tier structure provides a clear data flow: `services_v2` (producers) -> `relays` -> `services_v2` (consumers), all communicating over the `infra` bus.

## 3. Action Plan

1.  **Establish New Directory Tiers**:
    *   Create the `infra/`, `relays/`, and `libs/` directories within `backend_v2`.
    *   Reorganize existing source code into these new tiers. For example, the current `services_v2/adapters` will remain in `services_v2/`, but the conceptual role of the relays will be formalized by the new `relays/` directory.

2.  **Refactor an Initial Service (`kraken` collector)**:
    *   **Location**: Ensure the collector code resides at `services_v2/adapters/collectors/kraken/`.
    *   **Protocol Alignment**: Modify the collector's output to produce a `Vec<u8>` byte stream using the `TLVMessageBuilder`. This ensures adherence to the official binary protocol, making it compatible with the relay and bus architecture.
    *   **Configuration**: Refactor the service to receive its configuration (e.g., WebSocket URL, subscription pairs) externally, removing hardcoded values.

3.  **Update `backend_v2/README.md`**:
    *   The main `README.md` in `backend_v2` will be updated to document this three-tier architecture. It will explain the specific role of `infra`, `relays`, and `services_v2`, providing a clear guide for all future development.