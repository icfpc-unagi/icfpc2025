use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    // 1つのクライアントを使い回してコネクションプールを活用
    // DNS 解決を避けるため、ホスト名 -> IP:443 を固定解決
    let addr = SocketAddr::new("13.43.234.93".parse::<IpAddr>()?, 443);

    let client = Client::builder()
        .user_agent("icfpc2025-http-test/0.1")
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(16)
        .pool_idle_timeout(Duration::from_secs(90))
        .resolve("31pwr5t6ij.execute-api.eu-west-2.amazonaws.com", addr)
        .build()?;

    let host = "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com/";

    // 5回シーケンシャルに実行し、各リクエスト間の経過時間(ミリ秒)を表示
    let mut last = Instant::now();
    for i in 0..5 {
        let resp = client.get(host).send().await?;
        let status = resp.status();
        // ボディを読み切ってコネクションをプールへ返す
        let _ = resp.bytes().await?;

        let now = Instant::now();
        let elapsed_ms = now.duration_since(last).as_millis();
        println!("#{i}: {elapsed_ms} ms (status: {status})");
        last = now;
    }

    Ok(())
}
