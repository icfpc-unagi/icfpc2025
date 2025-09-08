use once_cell::sync::Lazy;
use std::net::{IpAddr, SocketAddr};

pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub static BLOCKING_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    // Pre-resolve the AWS API Gateway hostname to avoid DNS overhead and variance.
    // SNI/Host header remains the original hostname.
    let addr = SocketAddr::new(
        "13.43.234.93"
            .parse::<IpAddr>()
            .expect("invalid hardcoded IP"),
        443,
    );

    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(3))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(16)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .resolve("31pwr5t6ij.execute-api.eu-west-2.amazonaws.com", addr)
        .build()
        .expect("failed to build blocking reqwest client")
});
