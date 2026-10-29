# Dynu DDNS API Contract Specification

> [!NOTE]
> This document details the technical contract for Dynu's Dynamic DNS update endpoint.
> All findings below reflect empirical verification against Dynu production endpoints
> on October 29, 2026, executed via [`tests/contracts/test_dynu_contract.py`](file:///home/kiskaadee/Projects/active/ddns-monitor/tests/contracts/test_dynu_contract.py).

---

## 1. Remote Interface & Transport

* **Category:** Class A (Dedicated HTTP DDNS Protocol)
* **Base Endpoint:** `GET https://api.dynu.com/nic/update`
* **Transport:** HTTPS (Port 443) mandatory for credential safety; HTTP (Port 80) available for embedded/legacy devices.
* **Authentication Mechanisms:**
  1. *Query String Parameter:* `?password=<IP_UPDATE_PASSWORD>` (plaintext or MD5/SHA256 hash).
  2. *HTTP Basic Authentication:* `Authorization: Basic base64(username:password)`.
* **Primary Query Parameters:**
  * `hostname`: Fully qualified domain name (e.g., `node.dynu.net` or custom domain).
  * `myip`: Target IPv4 address.
  * `myipv6`: Target IPv6 address (or `no` to disable IPv6).
  * `password`: IP Update Password or account password when not using HTTP Basic Auth.
  * `offline`: `yes` marks the record as parked/offline.

---

## 2. Empirical Verification: Claims vs. Reality

Testing against live Dynu endpoints revealed several deviations from standard DynDNS2 specifications and generic documentation:

| Dimension / Test Case | Standard Claim | Observed Reality (HTTP 200 OK) | Classification |
|---|---|---|---|
| **Empty / Unauthenticated Request** | Returns `badauth` or `401 Unauthorized` | Returns string **`unknown`** | **DISCREPANCY** |
| **Invalid Password (IP Mismatch)** | Returns `badauth` | Returns string **`badauth`** | **CONFIRMED** |
| **Invalid Password (IP Match)** | Returns `badauth` | Short-circuits to **`nochg`** without checking password | **DISCREPANCY** |
| **Nonexistent Hostname (Query Auth)** | Returns `nohost` | Returns string **`badauth`** | **DISCREPANCY** |
| **Successful Idempotent Update** | Returns `nochg <ip>` | Returns bare string **`nochg`** | **DISCREPANCY** |
| **RFC 5737 Test IP (`198.51.100.42`)** | Sets record to test IP | Silently ignored; falls back to caller's WAN IP | **DISCREPANCY** |
| **Authoritative DNS Reflection** | Matches updated IP | Verified on `ns1.dynu.com` | **CONFIRMED** |

---

## 3. Behavioral Mechanisms & Analysis

### IP Match Short-Circuiting (The Idempotency Bypass)
If an update request arrives from an IP address that already matches the domain's existing `A` record, Dynu's gateway short-circuits the request and immediately returns `nochg` **before validating credentials**. 
* Consequence: Sending a bogus password for a domain that already resolves to your public IP returns `nochg`, not `badauth`.
* Authentication (`badauth`) is only enforced when the requested update would actually mutate the record (or when targeting another user's domain whose IP differs).

### Why Nonexistent Hostnames return `badauth`
Under query parameter authentication (`?hostname=<h>&password=<p>`), Dynu uses the `hostname` parameter to locate the corresponding user account and fetch the registered IP Update Password hash. If the hostname does not exist on Dynu's system, credential resolution fails *before* zone authorization occurs, causing the API to return `badauth` rather than `nohost`.

### Bare `nochg` Syntax
Standard DynDNS2 documentation dictates that idempotent updates return `nochg <ip>`. Dynu's server implementation frequently outputs only `nochg` with no following IP address. Upstream clients that expect an IP token following `nochg` fail to parse valid no-op responses.

### Bogon / Documentation IP Filtering
Dynu's edge gateway actively inspects the `myip` parameter. Non-routable addresses, bogons, and RFC 5737 documentation networks (`198.51.100.0/24`) are discarded. Rather than returning an error status, Dynu discards the parameter and defaults to the connection's public source IPv4 address. If the existing record already matches the source IP, Dynu returns `nochg`.

---

## 4. Adapter Implementation Requirements (Rust Engine)

The Dynu plugin ([`src/plugins/dynu.rs`](file:///home/kiskaadee/Projects/active/ddns-monitor/src/plugins/dynu.rs)) must observe the following contract invariants:

1. **Flexible `nochg` Matching:**
   Do not split response tokens expecting `nochg <ip>`. Treat any response starting with `nochg` as an idempotent success:
   ```rust
   if body.starts_with("good") || body.starts_with("nochg") {
       // Valid state reconciliation
   }
   ```
2. **Error Handling for `unknown`:**
   Account for `unknown` as an unauthenticated or missing-parameter error condition distinct from `badauth`.
3. **Pre-flight IP Validation:**
   The `ddns-monitor` core must validate public routability (filtering RFC 1918, RFC 5737, and loopback addresses) prior to invoking provider plugins, preventing Dynu's silent fallback behavior.

---

## 5. Untested Surfaces & Open Questions

- [ ] **Dual-Stack IPv6 Behavior:** Does supplying `myipv6` alongside `myip` update both `A` and `AAAA` records atomically, and does omitting `myipv6` preserve existing `AAAA` records?
- [ ] **HTTP Basic Auth Semantics:** Does authenticating via HTTP Basic Auth (`Authorization: Basic ...`) return `nohost` for nonexistent domains instead of `badauth`?
- [ ] **Abuse Rate Limiting:** What request frequency threshold triggers the `abuse` status code?
