//! Minimal reactive streams example
//!
//! Demonstrates:
//! - Creating streams from various sources
//! - Merging streams with select_all
//! - Processing events reactively (NO .await!)
//! - Clean shutdown

use futures::stream::{self, StreamExt};
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

#[derive(Debug)]
enum Event {
    Tick(u64),
    Message(&'static str),
    Shutdown,
}

#[tokio::main]
async fn main() {
    // NO .await anywhere in this code!

    println!("Starting reactive event loop...");

    // Create tick stream (simulates animation frames)
    let tick_stream = IntervalStream::new(tokio::time::interval(Duration::from_millis(100)))
        .enumerate()
        .map(|(i, _)| Event::Tick(i as u64));

    // Create message stream (simulates incoming data)
    let message_stream = stream::iter(vec![
        Event::Message("Hello"),
        Event::Message("Reactive"),
        Event::Message("World"),
    ]);

    // Create shutdown stream (after 1 second)
    let shutdown_stream = stream::once(async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Event::Shutdown
    });

    // Merge all streams
    let mut event_stream = stream::select_all(vec![
        Box::pin(tick_stream) as Pin<Box<dyn Stream<Item = Event> + Send>>,
        Box::pin(message_stream) as Pin<Box<dyn Stream<Item = Event> + Send>>,
        Box::pin(shutdown_stream) as Pin<Box<dyn Stream<Item = Event> + Send>>,
    ]);

    // Process events reactively
    while let Some(event) = event_stream.next().await {
        match event {
            Event::Tick(n) => println!("Tick {}", n),
            Event::Message(msg) => println!("Message: {}", msg),
            Event::Shutdown => {
                println!("Shutdown received!");
                break;
            }
        }
    }

    println!("Event loop complete!");
}
