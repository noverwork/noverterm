#[tokio::main]
async fn main() {
    if let Err(error) = backend::run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
