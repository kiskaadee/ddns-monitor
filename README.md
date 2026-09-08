# dynu-monitor

## Overview

`dynu-monitor` is a small Rust service that monitors the machine's public IPv4 address and triggers a Dynamic DNS update only when a verified IP change is detected.

The project exists primarily as a Rust learning project, but the resulting binary should be usable as a real component of a NixOS-based homelab.

The system follows a local-first approach:

1. Discover the current public IPv4 address.
2. Compare it against locally persisted state.
3. If unchanged, do nothing.
4. If changed, trigger `ddclient.service`.
5. Persist the new IP only after the update succeeds.

---

## Problem Statement

A DDNS updater can be invoked periodically regardless of whether the public IP address has changed. This can result in unnecessary requests to the DNS provider.

`dynu-monitor` acts as a local change detector in front of the existing DDNS updater.

The monitor should answer one question:

> Has the machine's public IPv4 address changed since the last successfully completed DDNS update?

If the answer is no, the DDNS updater should not be invoked.

If the answer is yes, the monitor should request an update and record the new address only after that update succeeds.

---

## Goals

### Functional Goals

* Discover the machine's public IPv4 address using external IP providers.
* Rotate through configured providers between executions.
* Validate that discovered addresses are valid public IPv4 addresses.
* Persist the last successfully updated IP address locally.
* Detect changes between the current and last successfully updated IP.
* Trigger `ddclient.service` when an IP change is detected.
* Preserve the previous successful IP if the DDNS update fails.
* Record successful and failed update attempts in an append-only history log.
* Exit with an appropriate non-zero status when the monitor cannot complete its operation.

### Learning Goals

The project should provide practical exposure to:

* Rust project structure and Cargo.
* Structs and enums.
* `Option<T>` and `Result<T, E>`.
* Ownership and borrowing.
* Traits.
* Generics.
* Dependency injection through constructor parameters.
* Error propagation with `?`.
* File I/O.
* JSON serialization.
* HTTP clients.
* Subprocess execution.
* Unit testing.

---

## Core Architecture

The application should separate orchestration from infrastructure.

```text
                    ┌───────────────┐
                    │    main.rs    │
                    │ composition   │
                    │     root      │
                    └───────┬───────┘
                            │
                            ▼
                    ┌───────────────┐
                    │ IpMonitorService │
                    │ application   │
                    │    logic      │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
          IpResolver   StateRepository  DdnsUpdater
              │             │             │
              ▼             ▼             ▼
            HTTP           JSON        systemctl

                            │
                            ▼
                     HistoryRepository
                            │
                            ▼
                          JSONL
```

The application service should contain the decision-making logic.

Infrastructure components should contain external concerns such as:

* HTTP.
* Filesystem access.
* JSON serialization.
* Subprocess execution.
* systemd interaction.

---

## Main Entities

### MonitorState

Represents the persisted state of the monitor.

```text
MonitorState
├── last_provider_index: Option<usize>
└── last_ip: Option<Ipv4Addr>
```

`last_ip` represents the last public IP for which a DDNS update completed successfully.

It must not be updated merely because a new IP was discovered.

---

### IpDiscoveryResult

Represents a successful public IP discovery.

```text
IpDiscoveryResult
├── ip: Ipv4Addr
├── provider: String
└── provider_index: usize
```

---

### HistoryEntry

Represents an event recorded for observability.

```text
HistoryEntry
├── timestamp
├── ip
├── status
└── details
```

The history log is an audit trail, not the primary source of runtime state.

---

## Roles and Responsibilities

### IpMonitorService

Responsible for the application's core workflow.

It should:

1. Load persisted state.
2. Determine which provider should be queried next.
3. Discover the current public IP.
4. Compare it with the previous successful IP.
5. Exit without updating if the IP is unchanged.
6. Trigger the DDNS updater if the IP changed.
7. Persist the new IP only after a successful update.
8. Record the update result.

It should not know:

* how HTTP requests are performed,
* how JSON files are read,
* how systemctl is invoked.

---

### IpResolver

Responsible for discovering a public IPv4 address.

Responsibilities:

* Query configured providers.
* Start with the requested provider index.
* Fall back through remaining providers if necessary.
* Validate returned addresses.
* Return information about the successful provider.

---

### StateRepository

Responsible for persistent monitor state.

Responsibilities:

* Load state.
* Save state atomically.
* Report persistence failures.

The state repository is the authoritative source of the monitor's runtime state.

---

### HistoryRepository

Responsible for recording monitor events.

Responsibilities:

* Append successful update events.
* Append failed update events.

The history log must not control normal application decisions.

---

### DdnsUpdater

Responsible for triggering the existing DDNS mechanism.

The initial implementation will invoke:

```text
systemctl start --wait ddclient.service
```

A successful invocation should represent a successful completion of the configured DDNS service.

The exact semantics of `ddclient.service` should be verified during NixOS integration.

---

## Core Workflow

```text
START
  │
  ▼
Load MonitorState
  │
  ▼
Select next IP provider
  │
  ▼
Discover public IPv4
  │
  ├── Failure
  │      │
  │      ▼
  │   Try fallback providers
  │
  └── Success
         │
         ▼
Compare with last_ip
         │
         ├── Same
         │      │
         │      ▼
         │   Save provider rotation state
         │      │
         │      ▼
         │     EXIT 0
         │
         └── Different / first run
                │
                ▼
           Trigger DDNS update
                │
                ├── Failure
                │      │
                │      ▼
                │ Record failure
                │      │
                │      ▼
                │ Keep previous last_ip
                │      │
                │      ▼
                │    EXIT != 0
                │
                └── Success
                       │
                       ▼
                Update last_ip
                       │
                       ▼
                  Persist state
                       │
                       ▼
                  Record success
                       │
                       ▼
                     EXIT 0
```

---

## Initial Technical Requirements

The first implementation should use:

* Rust stable.
* Cargo.
* A synchronous HTTP client.
* JSON for persistent state.
* JSON Lines for history.
* `std::process::Command` for systemd integration.

Likely dependencies:

```text
serde
serde_json
reqwest
thiserror
```

Async execution is intentionally not required for the first version.

---

## Non-Goals

The first version should not include:

* A web API.
* A GUI.
* A database.
* Authentication.
* Direct Dynu API integration.
* IPv6 support.
* Async networking.
* Concurrent provider queries.
* A dependency injection framework.
* A plugin system.
* Provider auto-discovery.
* Configuration hot reloading.

The project should remain a small system utility.

---

## Important Invariants

The following rules should remain true throughout the project:

1. `last_ip` means the last IP successfully submitted through the DDNS update process.

2. Discovering an IP does not modify `last_ip`.

3. A failed DDNS update must leave `last_ip` unchanged.

4. The monitor should not trigger DDNS when the discovered IP matches `last_ip`.

5. Provider rotation state may be persisted independently of IP changes.

6. Infrastructure implementations should be replaceable in tests.

---

## Suggested Implementation Milestones

### Milestone 1 — Domain

Implement:

* `MonitorState`.
* `IpDiscoveryResult`.
* IPv4 parsing and validation.
* Provider rotation logic.

No external dependencies.

### Milestone 2 — State

Add:

* JSON serialization.
* `StateRepository`.
* Atomic state persistence.

### Milestone 3 — Application

Implement:

* Traits for dependencies.
* `IpMonitorService`.
* Fake implementations for unit tests.

### Milestone 4 — HTTP

Implement:

* HTTP-based `IpResolver`.
* Provider fallback.
* Timeouts.

### Milestone 5 — DDNS

Implement:

* `SystemdDdnsUpdater`.
* Subprocess error handling.

### Milestone 6 — History

Implement:

* JSONL history.
* Success and failure event recording.

### Milestone 7 — NixOS Integration

Replace the existing Python monitor with the Rust binary and verify:

* Timer execution.
* State persistence.
* DDNS triggering.
* Failure recovery.

---

## Definition of Done

The first usable version is complete when:

* The binary can be executed manually.
* It can discover the machine's public IPv4 address.
* It persists monitor state.
* Repeated execution with the same IP does not invoke `ddclient`.
* A changed IP invokes `ddclient.service`.
* A failed DDNS update does not overwrite the previous successful IP.
* Provider failures fall back to another provider.
* Core application behavior is covered by unit tests without real HTTP, filesystem, or systemd dependencies.
* The binary can replace the existing Python implementation in the NixOS configuration.


