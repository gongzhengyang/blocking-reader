use blocking_reader::file::FileReadExt;
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let filepath = "/var/log/syslog";
    loop {
        let results = filepath
            .blocking_read_with_time_limit(&vec![], Duration::from_secs(30))
            .await
            .unwrap();
        if results.len() < 100 {
            println!("{results:?}");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
