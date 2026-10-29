import time
import urllib.error
import urllib.parse
import urllib.request

import dns.exception
import dns.resolver

DYNU_UPDATE_URL = "https://api.dynu.com/nic/update"

def _query_dynu(
    params: dict[str, str],
    headers: dict[str, str] | None = None,
    timeout: float = 10.0,
):
    query_string = urllib.parse.urlencode(params)
    url = f"{DYNU_UPDATE_URL}?{query_string}" if params else DYNU_UPDATE_URL
    req = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8", errors="replace").strip()
            return resp.status, body
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace").strip()
        return e.code, body

class TestDynuContract:
    """Empirical verification of the Dynu Class A DDNS API contract."""

    def test_reachability(self):
        """Observed: Empty/unauthenticated call returns 200 OK with 'unknown' (not badauth)."""
        status, body = _query_dynu({})
        assert status == 200, f"Expected HTTP 200, got {status} with body: {body}"
        assert body.startswith("unknown"), f"Expected body to start with 'unknown', got: {body}"

    def test_bad_credentials_on_foreign_host(self):
        """Observed: Invalid credentials for host whose IP differs from caller returns badauth."""
        params = {
            "hostname": "test.dynu.net",
            "password": "definitely_invalid_password_probe_12345",
        }
        status, body = _query_dynu(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body.startswith("badauth"), f"Expected 'badauth', got: {body}"

    def test_bad_credentials_short_circuit_on_matching_ip(self, dynu_config):
        """
        Observed: If caller's WAN IP already matches the domain's A record, Dynu
        short-circuits and returns 'nochg' without checking password validity.
        """
        params = {
            "hostname": dynu_config["hostname"],
            "password": "definitely_invalid_password_probe_12345",
        }
        status, body = _query_dynu(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body.startswith("nochg"), (
            f"Expected short-circuit 'nochg' when IP matches, got: {body}"
        )

    def test_nonexistent_hostname(self, dynu_config):
        """
        Observed: Under query auth (?password=...), nonexistent hostnames return 'badauth'
        instead of 'nohost' because Dynu uses the hostname to resolve the credential hash.
        """
        params = {
            "hostname": "nonexistent-test-node-unowned-12345.dynu.net",
            "myip": dynu_config["test_ipv4"],
            "password": dynu_config["password"],
        }
        status, body = _query_dynu(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body.startswith("badauth"), (
            f"Expected 'badauth' under query authentication for unowned host, got: {body}"
        )

    def test_valid_update_and_idempotency(self, dynu_config):
        """
        Observed:
        1. Updating returns 'good' or 'nochg'.
        2. Idempotent repetition returns bare 'nochg' (without an IP address appended).
        """
        target_ip = dynu_config["test_ipv4"]
        params = {
            "hostname": dynu_config["hostname"],
            "myip": target_ip,
            "password": dynu_config["password"],
        }

        # Step 1: Initial update
        status, body = _query_dynu(params)
        assert status == 200, f"Expected HTTP 200, got {status}"
        assert body.startswith(("good", "nochg")), (
            f"Expected 'good' or 'nochg', got: {body}"
        )

        # Safety pause to prevent abuse flagging
        time.sleep(2.0)

        # Step 2: Idempotent repetition
        status2, body2 = _query_dynu(params)
        assert status2 == 200, f"Expected HTTP 200, got {status2}"
        assert body2.startswith("nochg"), (
            f"Expected idempotent 'nochg' on second request, got: {body2}"
        )

    def test_authoritative_dns_propagation(self, dynu_config):
        """Observed: Authoritative nameservers answer for the configured hostname."""
        hostname = dynu_config["hostname"]

        resolver = dns.resolver.Resolver()
        try:
            ns_ips = [str(rdata) for rdata in dns.resolver.resolve("ns1.dynu.com", "A")]
            resolver.nameservers = ns_ips
        except (dns.exception.DNSException, OSError):
            pass  # fallback to default configured nameserver if ns1.dynu.com resolution fails

        answers = resolver.resolve(hostname, "A")
        resolved_ips = [str(rdata) for rdata in answers]
        assert len(resolved_ips) > 0, f"Expected at least one A record for {hostname}"
