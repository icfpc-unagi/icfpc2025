use once_cell::sync::Lazy;

pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub static BLOCKING_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("failed to build blocking reqwest client")
});
