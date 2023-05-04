mod proxy;
mod reporter;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    let bind_addr = args
        .nth(1)
        .ok_or("Specify a bind address as the first argument")?;
    let dest_addr = args
        .next()
        .ok_or("Specify a destination address as the second argument")?;

    println!("⚡️ sockgauge is forwarding {} -> {}", bind_addr, dest_addr);

    // Create a reporter and spawn a task to run it.
    let (reporter_handle, reporter_actor) = reporter::create();
    let reporter_join_handle = tokio::spawn(reporter_actor.run());

    // Run the proxy
    proxy::run(bind_addr, dest_addr, reporter_handle).await?;

    // Wait for the reporter task to finish.
    let _ = tokio::join!(reporter_join_handle);

    Ok(())
}
