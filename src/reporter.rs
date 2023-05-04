use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// Events that can be recorded.
pub enum Event {
    /// A socket was opened.
    Opened(SocketAddr),

    /// A socket was closed gracefully.
    ClosedGracefully(SocketAddr),

    /// A socket was closed with an error.
    ClosedWithError(SocketAddr, SocketCloseError),
}

/// The direction in which the error was encountered.
#[derive(Debug)]
pub enum Direction {
    /// The socket close error was encountered by forwarding client data to the server.
    ClientToServer,

    /// The socket close error was encountered by forwarding server data to the client.
    ServerToClient,
}

/// Errors pertaining to ungraceful socket closure.
#[derive(Debug)]
pub struct SocketCloseError(pub Direction, pub String);

/// Creates and returns a reporter actor as well as a handle for sending it messages.
pub fn create() -> (ReporterHandle, ReporterActor) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let actor = ReporterActor::new(receiver);
    let handle = ReporterHandle::new(sender);
    (handle, actor)
}

/// Used for sending events to the reporter.
#[derive(Clone)]
pub struct ReporterHandle {
    /// Used for sending events.
    sender: mpsc::UnboundedSender<Event>,
}

impl ReporterHandle {
    /// Creates a new handle.
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Reports the given event.
    pub fn report(&self, event: Event) {
        let _ = self.sender.send(event);
    }
}

/// The actor that processes the mailbox.
pub struct ReporterActor {
    /// The running count.
    count: u64,

    /// The receiver, used to consume the mailbox.
    receiver: mpsc::UnboundedReceiver<Event>,

    /// Map of socket addresses and the time they connected.
    connected_time: HashMap<SocketAddr, SystemTime>,
}

impl ReporterActor {
    /// Creates a new actor.
    fn new(receiver: mpsc::UnboundedReceiver<Event>) -> Self {
        Self {
            receiver,
            count: 0,
            connected_time: HashMap::with_capacity(1024),
        }
    }

    /// Runs the reporter actor mailbox processing loop. Must only be called once.
    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.receive(event)
        }
    }

    /// Receives an event and handles it.
    fn receive(&mut self, event: Event) {
        match event {
            Event::Opened(addr) => {
                // Increment the count.
                self.count += 1;

                // Record the time that they connected.
                self.connected_time.insert(addr, SystemTime::now());

                // Report the new connection.
                println!("🟢 {: >5} — new connection from {}", &self.count, &addr);
            }
            Event::ClosedGracefully(addr) => {
                // Handle socket close.
                let connected_duration = self.on_socket_closed(addr);

                // Report that the connection closed.
                println!(
                    "🔴 {: >5} — connection closed from {} (connected for {:?})",
                    &self.count, &addr, connected_duration
                );
            }
            Event::ClosedWithError(addr, err) => {
                // Handle socket close.
                let connected_duration = self.on_socket_closed(addr);

                // Report that the connection closed with an error.
                println!(
                    "🔴 {: >5} — connection closed from {}: ⚠️  {} (connected for {:?})",
                    &self.count, &addr, err, connected_duration
                );
            }
        }
    }

    /// Shared logic for when a socket is closed.
    fn on_socket_closed(&mut self, addr: SocketAddr) -> Duration {
        // Decrement the count.
        self.count -= 1;

        // Retrieve (and remove) the time that it connected so we can print the connection duration.
        let connected_at = self
            .connected_time
            .remove(&addr)
            .expect("No corresponding start time for socket?");

        // Return the connected duration.
        connected_at
            .elapsed()
            .expect("Error computing elapsed time?")
    }
}

/// Implement the `Error` trait.
impl std::error::Error for SocketCloseError {}

/// Implement formatting.
impl Display for SocketCloseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Direction::ClientToServer => {
                write!(f, "Error while forwarding client traffic to the server: ")?
            }
            Direction::ServerToClient => {
                write!(f, "Error while forwarding client traffic to the server: ")?
            }
        }

        write!(f, "{}", &self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let error = SocketCloseError(Direction::ClientToServer, "damn".to_string());
        assert_eq!(
            format!("{}", error),
            "Error while forwarding client traffic to the server: damn"
        );
    }
}
