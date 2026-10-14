pub mod api;

pub mod app;

pub mod event;

pub mod ui;

pub mod tui;

pub mod update;
use api::Client;
use app::App;
use color_eyre::Result;
use event::EventHandler;
use ratatui::{backend::CrosstermBackend, Terminal};
use tui::Tui;
use update::update;

fn main() -> Result<()> {
    color_eyre::install()?;

    let client = Client::new();
    let events = EventHandler::new(250, client.clone());
    let mut app = App::new(client, events.sender());

    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

    while !app.should_quit {
        tui.draw(&mut app)?;
        let event = tui.events.next()?;
        update(&mut app, event);
    }

    tui.exit()?;
    Ok(())
}
