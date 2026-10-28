use ddns_monitor::domain::traits::DdnsUpdater;
use ddns_monitor::plugins::dynu::DynuUpdater;
use std::net::Ipv4Addr;
use std::time::Duration;

#[test]
fn test_dynu_updater_good_response() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/nic/update")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("hostname".into(), "test.example.com".into()),
            mockito::Matcher::UrlEncoded("myip".into(), "198.51.100.5".into()),
            mockito::Matcher::UrlEncoded("password".into(), "secret_pass".into()),
        ]))
        .with_status(200)
        .with_body("good 198.51.100.5\n")
        .create();

    let endpoint = format!("{}/nic/update", server.url());
    let updater = DynuUpdater::with_endpoint(
        "test.example.com",
        "secret_pass",
        endpoint,
        Duration::from_secs(2),
    );

    let res = updater.update(Ipv4Addr::new(198, 51, 100, 5));
    assert!(res.is_ok());
}

#[test]
fn test_dynu_updater_nochg_response() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/nic/update")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("hostname".into(), "test.example.com".into()),
            mockito::Matcher::UrlEncoded("myip".into(), "198.51.100.5".into()),
            mockito::Matcher::UrlEncoded("password".into(), "secret_pass".into()),
        ]))
        .with_status(200)
        .with_body("nochg 198.51.100.5\n")
        .create();

    let endpoint = format!("{}/nic/update", server.url());
    let updater = DynuUpdater::with_endpoint(
        "test.example.com",
        "secret_pass",
        endpoint,
        Duration::from_secs(2),
    );

    let res = updater.update(Ipv4Addr::new(198, 51, 100, 5));
    assert!(res.is_ok());
}

#[test]
fn test_dynu_updater_badauth_response() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/nic/update")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("badauth\n")
        .create();

    let endpoint = format!("{}/nic/update", server.url());
    let updater = DynuUpdater::with_endpoint(
        "test.example.com",
        "wrong_pass",
        endpoint,
        Duration::from_secs(2),
    );

    let res = updater.update(Ipv4Addr::new(198, 51, 100, 5));
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Authentication failed"));
}

#[test]
fn test_dynu_updater_server_error() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/nic/update")
        .match_query(mockito::Matcher::Any)
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let endpoint = format!("{}/nic/update", server.url());
    let updater =
        DynuUpdater::with_endpoint("test.example.com", "pass", endpoint, Duration::from_secs(2));

    let res = updater.update(Ipv4Addr::new(198, 51, 100, 5));
    assert!(res.is_err());
}
