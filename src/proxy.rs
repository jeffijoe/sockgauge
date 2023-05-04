use crate::reporter::{Direction, Event, ReporterHandle, SocketCloseError};
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

/// Runs the proxy.
pub async fn run(
    bind_addr: String,
    dest_addr: String,
    reporter_handle: ReporterHandle,
) -> Result<(), std::io::Error> {
    // Bind to the socket.
    let listener = TcpListener::bind(bind_addr).await?;

    while let Ok((incoming, socket_addr)) = listener.accept().await {
        let reporter_handle = reporter_handle.clone();
        let dest_addr = dest_addr.clone();
        let proxy = async move {
            let result =
                handle_connection(incoming, &socket_addr, &dest_addr, reporter_handle).await;
            if let Err(err) = result {
                eprintln!("💥️ — proxying for socket {} failed: {}", &socket_addr, err)
            }
        };

        tokio::spawn(proxy);
    }

    Ok(())
}

/// Proxies the incoming socket to the destination.
async fn handle_connection(
    incoming: TcpStream,
    socket_addr: &SocketAddr,
    dest_addr: &String,
    reporter_handle: ReporterHandle,
) -> Result<(), Box<dyn Error>> {
    // Open a connection to the destination.
    let outbound = TcpStream::connect(dest_addr).await?;
    reporter_handle.report(Event::Opened(*socket_addr));

    // Wait for the proxying to complete (either socket closes).
    let transfer_result = transfer(incoming, outbound).await;

    if let Err(err) = transfer_result {
        reporter_handle.report(Event::ClosedWithError(*socket_addr, err));
        return Ok(());
    }

    // Report that the connection closed.
    reporter_handle.report(Event::ClosedGracefully(*socket_addr));
    Ok(())
}

/// Runs the actual proxying of a socket.
async fn transfer(
    mut incoming: TcpStream,
    mut outbound: TcpStream,
) -> Result<(), SocketCloseError> {
    // Split the streams into read and write halves.
    let (mut read_inbound, mut write_inbound) = incoming.split();
    let (mut read_outbound, mut write_outbound) = outbound.split();

    // Connect the client reader to the server writer.
    // That is, whenever we receive data from the client, we forward it to the server.
    let client_to_server = async {
        tokio::io::copy(&mut read_inbound, &mut write_outbound)
            .await
            .map_err(|e| map_io_error(Direction::ClientToServer, e))?;
        write_outbound
            .shutdown()
            .await
            .map_err(|e| map_io_error(Direction::ClientToServer, e))
    };

    // Connect the server reader to the client writer.
    // That is, whenever we receive data from the server, we forward it to the client.
    let server_to_client = async {
        tokio::io::copy(&mut read_outbound, &mut write_inbound)
            .await
            .map_err(|e| map_io_error(Direction::ServerToClient, e))?;
        write_inbound
            .shutdown()
            .await
            .map_err(|e| map_io_error(Direction::ServerToClient, e))
    };

    // Poll both tasks.
    tokio::try_join!(client_to_server, server_to_client)?;

    Ok(())
}

/// Maps IO error to a `SocketCloseError`.
fn map_io_error(direction: Direction, err: std::io::Error) -> SocketCloseError {
    SocketCloseError(direction, err.to_string())
}
