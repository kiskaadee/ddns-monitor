# DuckDNS API Contract Specification

> [!NOTE]
> This document details the technical contract for DuckDNS's Dynamic DNS update endpoint.
> All findings below reflect empirical verification against DuckDNS production endpoints
> on October 29, 2026, executed via [`tests/contracts/test_duckdns_contract.py`](file:///home/kiskaadee/Projects/active/ddns-monitor/tests/contracts/test_duckdns_contract.py).

---

## 1. Remote Interface & Transport

* **Category:** Class A (Dedicated HTTP DDNS Protocol)
* **Base Endpoint:** `GET https://www.duckdns.org/update`
* **Transport:** HTTPS (Port 443) recommended; HTTP (Port 80) supported.
* **Authentication Mechanism:** Account-level UUID security token embedded directly into the query parameters (`?token=<UUID>`). Does not support or require HTTP Authorization headers.
* **Targeting Scope:** Target subdomains are scoped directly under `.duckdns.org`. The API takes the subdomain prefix (e.g. `myhost` for `myhost.duckdns.org`).
* **Primary Query Parameters:**
  * `domains`: Single subdomain prefix or comma-separated list (e.g., `node1,node2`).
  * `token`: Account UUID token.
  * `ip`: Explicit IPv4 address (optional; auto-detected if omitted).
  * `ipv6`: Explicit IPv6 address (optional; dual-stack update).
  * `verbose`: When set to `true`, returns a multiline diagnostic block instead of bare `OK`.
  * `clear`: `true` wipes the record set for the given domain.

---

## 2. Empirical Verification: Claims vs. Reality

Testing against live DuckDNS production endpoints verified the custom query protocol and its verbose diagnostic dialect:

| Dimension / Test Case | Standard Claim | Observed Reality (HTTP 200 OK) | Classification |
|---|---|---|---|
| **Empty / Unauthenticated Request** | Returns `KO` | Returns string **`KO`** | **CONFIRMED** |
| **Invalid Token** | Returns `KO` | Returns string **`KO`** | **CONFIRMED** |
| **Unowned Domain** | Returns `KO` | Returns string **`KO`** | **CONFIRMED** |
| **Standard Mode Update** | Returns `OK` | Returns string **`OK`** | **CONFIRMED** |
| **Standard Mode Idempotency** | Returns `OK` (no-op blind) | Returns string **`OK`** (no distinction) | **CONFIRMED** |
| **Verbose Mode (`verbose=true`)** | Returns multiline block | Returns 4-line diagnostic block | **CONFIRMED** |
| **Authoritative DNS Reflection** | Matches updated IP | Verified on `ns1.duckdns.org` | **CONFIRMED** |

---

## 3. Behavioral Mechanisms & Analysis

### Binary Response Model
Unlike DynDNS2-compliant providers (Dynu, No-IP) that differentiate between `badauth`, `nohost`, and `abuse`, DuckDNS uses a binary response model:
* **`OK`**: Request accepted and processed.
* **`KO`**: Request failed (invalid token, domain unowned, missing parameters).

The engine cannot programmatically distinguish an invalid token from an unowned domain without out-of-band diagnostics.

### Verbose Diagnostic Block Anatomy
When `verbose=true` is appended to the query parameters, the response format changes from a single token into a structured multiline text payload:

```text
Line 1: OK                   <- Overall status (OK or KO)
Line 2: 186.168.87.15        <- Applied IPv4 address
Line 3:                      <- Applied IPv6 address (blank if unset)
Line 4: UPDATED              <- State flag: "UPDATED" or "NOCHANGE"
```

* **First Update (Mutating):** Line 4 outputs `UPDATED`.
* **Second Update (Idempotent):** Line 4 outputs `NOCHANGE`.

### Subdomain Prefix Stripping
DuckDNS strictly expects the subdomain prefix in `domains=` rather than a Fully Qualified Domain Name (FQDN). Passing `myhost.duckdns.org` rather than `myhost` causes DuckDNS to attempt updating `myhost.duckdns.org.duckdns.org`, returning `KO`.

---

## 4. Adapter Implementation Requirements (Rust Engine)

The DuckDNS plugin (`src/plugins/duckdns.rs`) must observe the following invariants:

1. **Always Request Verbose Mode:**
   To maintain engine idempotency metrics, always append `&verbose=true` to update requests. Parse Line 4 to differentiate state mutation (`UPDATED`) from redundant synchronization (`NOCHANGE`):
   ```rust
   let lines: Vec<&str> = body.lines().collect();
   if lines.get(0) == Some(&"OK") {
       let changed = lines.get(3) == Some(&"UPDATED");
       // Record state accordingly
   }
   ```
2. **FQDN Normalization:**
   Strip `.duckdns.org` suffixes from user-supplied hostname configuration before building the query string:
   ```rust
   let domain_prefix = hostname.trim_end_matches(".duckdns.org");
   ```
3. **No Abuse Counter on Idempotent Hits:**
   DuckDNS does not rate-limit or flag rapid unchanged polling as abuse in the same aggressive manner as DynDNS2 providers, but the local engine cache must still avoid unnecessary outbound calls.

---

## 5. Untested Surfaces & Open Questions

- [ ] **Dual-Stack IPv6 Behavior:** Does passing `ip` and `ipv6` simultaneously populate both lines in verbose mode without overwriting?
- [ ] **Batch Updates:** Does `domains=sub1,sub2` update multiple records atomically under a single token?
- [ ] **Record Clearing:** Does `clear=true` remove both `A` and `AAAA` records completely?
