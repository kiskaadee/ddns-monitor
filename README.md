# ddns-monitor

A provider-agnostic Dynamic DNS monitor written in Rust.

Monitors the machine's public WAN IPv4 address and, when a change is detected, notifies
all configured DDNS provider subscribers. Designed to replace `ddclient` on NixOS homelabs
and to work on any standard Linux system.

---

## Why this exists

DDNS updaters like `ddclient` couple the IP-detection step to a single provider. When that
provider has an outage, the entire update chain breaks — even though your IP may not have
changed and other providers could accept the update.

`ddns-monitor`:

- Separates **IP discovery** (round-robin across multiple checkers) from **DNS updating**
  (fan-out to multiple DDNS provider plugins).
- Stores the last-known-good IP locally and skips updates when nothing has changed.
- Notifies all configured subscribers on a detected change.
- Runs as a systemd oneshot timer, consuming no resources between invocations.

---

## Status

**Phase 0 — in progress.** Laying foundations: flake, documentation, module structure.

See [`ROADMAP.md`](./ROADMAP.md) for the full plan.

---

## Quick Start (development)

Prerequisites: Nix with flakes enabled. No system OpenSSL required — TLS is handled by bundled rustls.

```bash
# Enter the development shell (downloads pinned Rust toolchain)
nix develop

# Build
cargo build

# Run tests
cargo nextest run

# Live-reload on save
cargo watch -x check
```

---

## Architecture (target)

```
main.rs (composition root)
└── IpMonitorService (orchestration)
    ├── IpResolver      — discovers WAN IP via rotating external checkers
    ├── StateRepository — persists last-known-good IP as JSON
    ├── NotificationBus — fans out to all configured DDNS updaters
    │   ├── DynuUpdater       (Phase 6)
    │   └── «future plugins»
    └── HistoryRepository — append-only JSONL audit log
```

The application service contains all decision logic. Infrastructure components (HTTP, fs,
subprocess) are hidden behind traits and swappable in tests.

---

## Configuration

The binary reads from (in priority order):

1. NixOS module options (declarative, build-time injection)
2. `~/.config/ddns-monitor/config.toml` (runtime, for non-NixOS)

Example:

```toml
[ip_providers]
urls = [
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
]
timeout_secs = 5

[[subscribers]]
provider = "dynu"
domain   = "example.dyndns.org"
credentials_env = "DYNU_PASSWORD"
```

> **Never store plaintext credentials in `config.toml`.** Use env vars or sops-nix secrets.

---

## NixOS

The NixOS module (Phase 8) will:

- Build the binary as a Nix derivation.
- Expose `services.ddns-monitor.subscribers` as a list of option sets.
- Inject credentials from `sops.secrets` via `LoadCredential`.
- Run as a oneshot systemd timer (30-minute default, configurable).

---

## Invariants

1. `last_ip` = the IP from the last fully successful fan-out (all subscribers succeeded).
2. Discovering an IP does **not** update `last_ip`.
3. A partial or total update failure leaves `last_ip` unchanged.
4. The monitor does not fire DDNS when the discovered IP matches `last_ip`.
5. Provider rotation state may be persisted independently of IP changes.
6. All infrastructure dependencies are injectable via traits (testable without network/fs).

---

## License

[Unlicense](./UNLICENSE) — public domain.
