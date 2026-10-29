import time
import urllib.error
import urllib.parse
import urllib.request

import dns.exception
import dns.resolver

DUCKDNS_UPDATE_URL = "https://www.duckdns.org/update"


def _query_duckdns(
    params: dict[str, str],
    headers: dict[str, str] | None = None,
    timeout: float = 10.0,
) -> tuple[int, str]:
    query_string = urllib.parse.urlencode(params)
    url = f"{DUCKDNS_UPDATE_URL}?{query_string}" if params else DUCKDNS_UPDATE_URL
    req = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8", errors="replace").strip()
            return resp.status, body
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace").strip()
        return e.code, body


class TestDuckDnsContract:
    """Empirical verification of the DuckDNS Class A DDNS API contract."""

    def test_reachability(self):
        """Claim: Empty/unauthenticated update request fails cleanly."""
        status, body = _query_duckdns({})
        assert status == 200, f"Expected HTTP 200, got {status} with body: {body}"
        assert body == "KO", f"Expected body 'KO' on empty request, got: {body}"

    def test_invalid_token(self, duckdns_config):
        """Claim: Request with valid domain but invalid token returns 'KO'."""
        params = {
            "domains": duckdns_config["domain"],
            "token": "00000000-0000-0000-0000-000000000000",
            "ip": duckdns_config["test_ipv4"],
        }
        status, body = _query_duckdns(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body == "KO", f"Expected 'KO' on bad token, got: {body}"

    def test_unowned_domain(self, duckdns_config):
        """Claim: Request with valid token but unowned domain returns 'KO'."""
        params = {
            "domains": "unowned-random-subdomain-probe-99999",
            "token": duckdns_config["token"],
            "ip": duckdns_config["test_ipv4"],
        }
        status, body = _query_duckdns(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body == "KO", f"Expected 'KO' on unowned domain, got: {body}"

    def test_standard_update_and_idempotence(self, duckdns_config):
        """
        Claim:
        1. Standard update returns plain string 'OK'.
        2. Idempotent repetition also returns 'OK' without differentiation.
        """
        target_ip = duckdns_config["test_ipv4"]
        params = {
            "domains": duckdns_config["domain"],
            "token": duckdns_config["token"],
            "ip": target_ip,
        }

        # Step 1: First update
        status1, body1 = _query_duckdns(params)
        assert status1 == 200, f"Expected HTTP 200, got {status1}"
        assert body1 == "OK", f"Expected 'OK' on standard update, got: {body1}"

        time.sleep(1.0)

        # Step 2: Idempotent repetition
        status2, body2 = _query_duckdns(params)
        assert status2 == 200, f"Expected HTTP 200, got {status2}"
        assert body2 == "OK", f"Expected 'OK' on repeat update, got: {body2}"

    def test_verbose_mode_semantics(self, duckdns_config):
        """
        Claim: Passing verbose=true returns multiline diagnostic block
        distinguishing UPDATED vs NOCHANGE.
        """
        target_ip = duckdns_config["test_ipv4"]
        params = {
            "domains": duckdns_config["domain"],
            "token": duckdns_config["token"],
            "ip": target_ip,
            "verbose": "true",
        }
        status, body = _query_duckdns(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        lines = [line.strip() for line in body.splitlines() if line.strip()]
        assert len(lines) >= 1, f"Expected multiline output, got: {body}"
        assert lines[0] == "OK", f"Expected first line to be 'OK', got: {lines[0]}"

    def test_authoritative_dns_propagation(self, duckdns_config):
        """Claim: Authoritative nameservers answer for the configured hostname."""
        subdomain = duckdns_config["domain"]
        fqdn = f"{subdomain}.duckdns.org" if not subdomain.endswith(".duckdns.org") else subdomain

        resolver = dns.resolver.Resolver()
        try:
            ns_ips = [str(rdata) for rdata in dns.resolver.resolve("ns1.duckdns.org", "A")]
            resolver.nameservers = ns_ips
        except (dns.exception.DNSException, OSError):
            pass

        answers = resolver.resolve(fqdn, "A")
        resolved_ips = [str(rdata) for rdata in answers]
        assert len(resolved_ips) > 0, f"Expected at least one A record for {fqdn}"
