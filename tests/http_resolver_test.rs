use ddns_monitor::domain::traits::IpResolver;
use ddns_monitor::infra::http_resolver::HttpIpResolver;
use std::net::Ipv4Addr;
use std::time::Duration;

#[test]
fn test_http_resolver_success() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/ip")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("93.184.216.34\n")
        .create();

    let url = format!("{}/ip", server.url());
    let resolver = HttpIpResolver::new(vec![url.clone()], Duration::from_secs(2));

    let res = resolver.resolve(None).expect("resolve succeeded");
    assert_eq!(res.ip, Ipv4Addr::new(93, 184, 216, 34));
    assert_eq!(res.provider, url);
    assert_eq!(res.provider_index, 0);
}

#[test]
fn test_http_resolver_fallback_on_invalid_ip() {
    let mut server1 = mockito::Server::new();
    let _m1 = server1
        .mock("GET", "/ip")
        .with_status(200)
        .with_body("192.168.1.1\n") // Private IP - invalid for public WAN
        .create();

    let mut server2 = mockito::Server::new();
    let _m2 = server2
        .mock("GET", "/ip")
        .with_status(200)
        .with_body("198.51.100.22\n") // Valid public IP
        .create();

    let url1 = format!("{}/ip", server1.url());
    let url2 = format!("{}/ip", server2.url());

    let resolver = HttpIpResolver::new(vec![url1, url2.clone()], Duration::from_secs(2));

    let res = resolver.resolve(None).expect("resolve fallback succeeded");
    assert_eq!(res.ip, Ipv4Addr::new(198, 51, 100, 22));
    assert_eq!(res.provider, url2);
    assert_eq!(res.provider_index, 1);
}

#[test]
fn test_http_resolver_rotation_starting_point() {
    let mut server1 = mockito::Server::new();
    let _m1 = server1
        .mock("GET", "/ip")
        .with_status(200)
        .with_body("93.184.216.1\n")
        .create();

    let mut server2 = mockito::Server::new();
    let _m2 = server2
        .mock("GET", "/ip")
        .with_status(200)
        .with_body("93.184.216.2\n")
        .create();

    let url1 = format!("{}/ip", server1.url());
    let url2 = format!("{}/ip", server2.url());

    let resolver = HttpIpResolver::new(vec![url1, url2.clone()], Duration::from_secs(2));

    // last_index was 0, so next rotation MUST query server2 (index 1) first!
    let res = resolver.resolve(Some(0)).expect("resolve rotated");
    assert_eq!(res.ip, Ipv4Addr::new(93, 184, 216, 2));
    assert_eq!(res.provider, url2);
    assert_eq!(res.provider_index, 1);
}

#[test]
fn test_http_resolver_all_failed() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/ip")
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let url = format!("{}/ip", server.url());
    let resolver = HttpIpResolver::new(vec![url], Duration::from_secs(1));

    let res = resolver.resolve(None);
    assert!(res.is_err());
}
