//! Yollayah TUI Entry Point
//!
//! Launches the terminal UI for Yollayah, the cutest AI companion.
//!
//! Usage:
//!   yollayah-tui [OPTIONS]
//!
//! Options:
//!   --ollama-host <HOST>  Ollama host (default: localhost)
//!   --ollama-port <PORT>  Ollama port (default: 11434)
//!   --server-mode         Connect via ai-way server instead of direct Ollama

use std::io;
use std::panic;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use yollayah_tui::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Check if we have a TTY before attempting initialization
    use std::io::IsTerminal;

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        eprintln!("❌ Error: yollayah-tui requires a terminal (TTY)");
        eprintln!("");
        eprintln!("This usually means:");
        eprintln!("  • Running in a non-interactive environment (CI, container)");
        eprintln!("  • SSH without -t flag");
        eprintln!("  • Piped stdin/stdout");
        eprintln!("");
        eprintln!("Solutions:");
        eprintln!("  • Run interactively: ./yollayah.sh");
        eprintln!("  • Or with toolbox: toolbox run --directory $PWD ./yollayah.sh");
        eprintln!("  • Or with script: script -c './yollayah.sh' /dev/null");
        std::process::exit(1);
    }

    // Set up panic hook to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before printing panic
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run the app
    let result = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Propagate any errors
    result
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new().await?;
    app.run(terminal).await?;

    // Show goodbye message after TUI closes
    if let Some(goodbye) = app.goodbye() {
        // Print with Yollayah styling (magenta)
        println!("\n\x1b[35mYollayah:\x1b[0m {}\n", goodbye);
    }

    Ok(())
}
