use once_cell::sync::Lazy;

pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub static BLOCKING_CLIENT: Lazy<reqwest::blocking::Client> =
    Lazy::new(reqwest::blocking::Client::new);
