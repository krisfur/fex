mod app;
mod provider;
mod providers;
mod ui;
mod util;

use std::io::{self, Write as _};

use clap::Parser;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{App, AppAction};

#[derive(Parser)]
#[command(name = "fex", about = "A TUI package search tool", version)]
struct Cli {
    /// Use a specific package provider
    #[arg(short = 'p', long = "provider", value_name = "PROVIDER")]
    provider: Option<String>,

    /// List available providers and exit
    #[arg(short = 'l', long = "list")]
    list: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list {
        let available = providers::get_available_providers();
        if available.is_empty() {
            println!("No supported package managers found.");
        } else {
            println!("Available providers:");
            for (name, _) in available {
                println!("  {name}");
            }
        }
        return;
    }

    let provider = if let Some(name) = cli.provider {
        match providers::create_provider(&name) {
            Some(p) if p.is_available() => p,
            Some(_) => {
                eprintln!("Provider '{name}' is not available on this system.");
                std::process::exit(1);
            }
            None => {
                eprintln!("Unknown provider '{name}'. Use -l to list available providers.");
                std::process::exit(1);
            }
        }
    } else {
        match providers::auto_detect_provider() {
            Some(p) => p,
            None => {
                eprintln!("No supported package manager found.");
                std::process::exit(1);
            }
        }
    };

    if let Err(e) = run(provider) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run(provider: crate::provider::BoxedProvider) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(provider);

    let result = run_loop(&mut app, &mut terminal);

    // Always restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;

    result.map_err(Into::into)
}

fn run_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    loop {
        match app.run(terminal)? {
            AppAction::Quit => break,
            AppAction::Install => {
                install_package(app, terminal)?;
            }
        }
    }
    Ok(())
}

fn install_package(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    if app.packages.is_empty() || app.selected >= app.packages.len() {
        return Ok(());
    }

    let pkg = &app.packages[app.selected];
    let cmd = app.provider.install_command(pkg);
    let pkg_name = pkg.name.clone();
    let pkg_source = pkg.source.clone();
    let pkg_idx = app.selected;

    // Leave TUI
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;

    // Run the install command
    println!("\nInstalling {pkg_name} from {pkg_source}...\n");
    let status = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .status()
        .ok();

    // Wait for user acknowledgement
    println!("\nPress Enter to return...");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    // Re-enter TUI
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, Hide)?;
    terminal.clear()?;

    // Update installed status
    let success = status.map(|s| s.success()).unwrap_or(false);
    if success {
        app.packages[pkg_idx].installed = true;
        app.status_message = format!("Successfully installed {pkg_name}");
    } else {
        app.status_message = format!("Installation of {pkg_name} may have failed");
    }

    Ok(())
}
