use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use color_eyre::Result;
use ratatui::crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};

use crate::api::{Addon, Bean, Client, Drink, Milk, Order};

#[derive(Debug, Clone)]
pub struct MenuData {
    pub drinks: Vec<Drink>,
    pub beans: Vec<Bean>,
    pub milks: Vec<Milk>,
    pub addons: Vec<Addon>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    MenuLoaded(Result<MenuData, String>),
    OrdersLoaded(Result<Vec<Order>, String>),
    OrderSubmitted(Result<Vec<Order>, String>),
    StatusChanged(Result<Order, String>),
}

pub const ORDERS_POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct EventHandler {
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
    #[allow(dead_code)]
    input_handler: thread::JoinHandle<()>,
    #[allow(dead_code)]
    poll_handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: u64, client: Client) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();

        let input_handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for event") {
                        match event::read().expect("unable to read event") {
                            CrosstermEvent::Key(e) => {
                                if e.kind == event::KeyEventKind::Press {
                                    sender.send(Event::Key(e))
                                } else {
                                    Ok(())
                                }
                            }
                            CrosstermEvent::Mouse(e) => sender.send(Event::Mouse(e)),
                            CrosstermEvent::Resize(w, h) => sender.send(Event::Resize(w, h)),
                            _ => Ok(()),
                        }
                        .expect("failed to send terminal event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.send(Event::Tick).expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };

        let poll_handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let menu = (
                    client.fetch_drinks(),
                    client.fetch_beans(),
                    client.fetch_milks(),
                    client.fetch_addons(),
                );
                let menu_event = match menu {
                    (Ok(drinks), Ok(beans), Ok(milks), Ok(addons)) => Event::MenuLoaded(Ok(MenuData {
                        drinks,
                        beans,
                        milks,
                        addons,
                    })),
                    (d, b, m, a) => Event::MenuLoaded(Err(format!(
                        "menu load failed: {}",
                        [d.err(), b.err(), m.err(), a.err()]
                            .into_iter()
                            .flatten()
                            .next()
                            .map(|e| e.to_string())
                            .unwrap_or_default()
                    ))),
                };
                if sender.send(menu_event).is_err() {
                    return;
                }

                loop {
                    let result = client.fetch_orders().map_err(|e| e.to_string());
                    if sender.send(Event::OrdersLoaded(result)).is_err() {
                        return;
                    }
                    thread::sleep(ORDERS_POLL_INTERVAL);
                }
            })
        };

        Self {
            sender,
            receiver,
            input_handler,
            poll_handler,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<Event> {
        self.sender.clone()
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
