use std::net::Ipv4Addr;

/// Checks if an IPv4 address is a valid, publicly routable WAN address.
///
/// Excludes RFC 1918 private ranges, RFC 6598 carrier-grade NAT,
/// loopback, link-local, multicast, and broadcast addresses.
pub fn is_valid_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    let (a, b) = (octets[0], octets[1]);

    // Unspecified / "This host on this network" (0.0.0.0/8)
    if a == 0 {
        return false;
    }

    // Loopback (127.0.0.0/8)
    if a == 127 {
        return false;
    }

    // RFC 1918 Private networks
    // 10.0.0.0/8
    if a == 10 {
        return false;
    }
    // 172.16.0.0/12
    if a == 172 && (16..=31).contains(&b) {
        return false;
    }
    // 192.168.0.0/16
    if a == 192 && b == 168 {
        return false;
    }

    // RFC 6598 Carrier-Grade NAT (100.64.0.0/10)
    if a == 100 && (64..=127).contains(&b) {
        return false;
    }

    // RFC 3927 Link-local (169.254.0.0/16)
    if a == 169 && b == 254 {
        return false;
    }

    // Multicast (224.0.0.0/4: 224..=239) and Reserved / Broadcast (240.0.0.0/4: 240..=255)
    if a >= 224 {
        return false;
    }

    true
}

/// Computes the next provider index using round-robin rotation.
///
/// Returns `None` if `provider_count` is 0.
pub fn next_provider_index(last_index: Option<usize>, provider_count: usize) -> Option<usize> {
    if provider_count == 0 {
        return None;
    }

    match last_index {
        Some(idx) => Some((idx + 1) % provider_count),
        None => Some(0),
    }
}
