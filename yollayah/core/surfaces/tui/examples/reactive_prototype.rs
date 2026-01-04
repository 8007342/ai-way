//! Full reactive TUI prototype
//!
//! This prototype demonstrates:
//! - Real terminal event handling (crossterm)
//! - Multiple merged event streams
//! - 10 FPS rendering loop
//! - Message processing (simulated conductor)
//! - Clean shutdown
//! - Performance monitoring
//!
//! NO .await calls in event processing!

use crossterm::{
    event::{Event as CrosstermEvent, EventStream, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::stream::{self, StreamExt};
use futures::Stream;
use std::io::Write;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio_stream::wrappers::IntervalStream;

#[derive(Debug)]
enum AppEvent {
    Terminal(CrosstermEvent),
    Tick,
    Message(String),
    Shutdown,
}

struct AppState {
    running: bool,
    frame_count: u64,
    message_count: u64,
    event_count: u64,
    start_time: Instant,
}

impl AppState {
    fn new() -> Self {
        Self {
            running: true,
            frame_count: 0,
            message_count: 0,
            event_count: 0,
            start_time: Instant::now(),
        }
    }

    fn handle_event(&mut self, event: AppEvent) {
        self.event_count += 1;

        match event {
            AppEvent::Terminal(CrosstermEvent::Key(key)) => {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    println!("\n\nShutdown requested by user");
                    self.running = false;
                }
            }
            AppEvent::Tick => {
                self.frame_count += 1;
            }
            AppEvent::Message(msg) => {
                self.message_count += 1;
                println!("Message {}: {}", self.message_count, msg);
            }
            AppEvent::Shutdown => {
                println!("\n\nShutdown signal received");
                self.running = false;
            }
            _ => {}
        }
    }

    fn render(&self) {
        if self.frame_count % 10 == 0 {
            // Print stats every second
            let elapsed = self.start_time.elapsed();
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            let events_per_sec = self.event_count as f64 / elapsed.as_secs_f64();

            print!(
                "\r[Frame: {} | FPS: {:.1} | Events: {} | Evt/sec: {:.0} | Messages: {}]",
                self.frame_count, fps, self.event_count, events_per_sec, self.message_count
            );
            std::io::stdout().flush().unwrap();
        }
    }
}

fn create_terminal_stream() -> impl Stream<Item = AppEvent> {
    EventStream::new().map(|result| {
        result
            .map(AppEvent::Terminal)
            .unwrap_or_else(|_| AppEvent::Message("Terminal error".to_string()))
    })
}

fn create_tick_stream() -> impl Stream<Item = AppEvent> {
    IntervalStream::new(tokio::time::interval(Duration::from_millis(100)))
        .map(|_| AppEvent::Tick)
}

fn create_message_stream() -> impl Stream<Item = AppEvent> {
    // Simulate high-throughput message stream (100 msgs/sec)
    IntervalStream::new(tokio::time::interval(Duration::from_millis(10)))
        .take(1000) // 1000 messages total (10 seconds)
        .enumerate()
        .map(|(i, _)| AppEvent::Message(format!("Token_{}", i)))
}

fn create_shutdown_stream() -> impl Stream<Item = AppEvent> {
    // Shutdown after 10 seconds
    stream::once(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        AppEvent::Shutdown
    })
}

fn create_event_pipeline() -> impl Stream<Item = AppEvent> {
    use futures::stream::select_all;

    select_all(vec![
        Box::pin(create_terminal_stream()) as Pin<Box<dyn Stream<Item = AppEvent> + Send>>,
        Box::pin(create_tick_stream()) as Pin<Box<dyn Stream<Item = AppEvent> + Send>>,
        Box::pin(create_message_stream()) as Pin<Box<dyn Stream<Item = AppEvent> + Send>>,
        Box::pin(create_shutdown_stream()) as Pin<Box<dyn Stream<Item = AppEvent> + Send>>,
    ])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    enable_raw_mode()?;
    println!("Reactive TUI Prototype");
    println!("======================");
    println!("Press 'q' or ESC to quit, or wait 10 seconds for auto-shutdown");
    println!();

    // Create app state
    let mut app = AppState::new();

    // Create reactive event pipeline
    let mut event_stream = create_event_pipeline();

    // Main event loop - NO .await in event processing!
    while app.running {
        if let Some(event) = event_stream.next().await {
            app.handle_event(event);
            app.render();
        } else {
            break; // Stream exhausted
        }
    }

    // Cleanup
    disable_raw_mode()?;

    // Print final stats
    let elapsed = app.start_time.elapsed();
    println!("\n\nPrototype Statistics:");
    println!("====================");
    println!("Total runtime: {:.2}s", elapsed.as_secs_f64());
    println!("Total frames: {}", app.frame_count);
    println!(
        "Average FPS: {:.1}",
        app.frame_count as f64 / elapsed.as_secs_f64()
    );
    println!("Total events: {}", app.event_count);
    println!(
        "Events/sec: {:.0}",
        app.event_count as f64 / elapsed.as_secs_f64()
    );
    println!("Messages processed: {}", app.message_count);

    Ok(())
}
