# buppers-terminal-gui

A terminal "bar terminal" for the Buppers coffee shop, built with [Ratatui](https://ratatui.rs). It has two screens:

- **Order**: browse the menu, customize drinks (milk, hot/iced, add-ons), build a cart, and send it to the bar under a customer name.
- **Board**: a live kanban of orders by status (New → Brewing → Ready → Picked up). You can move an order to the next status from the keyboard.

It talks to the `buppers/*` services on the Buppers FeathersJS backend (`feathers-api`).

## Requirements

- Rust 1.88 or newer (the crate uses edition 2024 and `let` chains)
- A running Buppers backend that serves `buppers/drinks`, `buppers/beans`, `buppers/milks`, `buppers/addons` and `buppers/orders`
- A terminal with true-color support, for the intended color palette

## Getting started

```bash
git clone https://github.com/justinszaro/buppers-terminal-gui.git
cd buppers-terminal-gui
cargo run --release
```

The app connects to `http://localhost:3030` by default. To use another backend, set `BASE_URL`:

```bash
BASE_URL=https://api.example.com cargo run --release
```

On startup it loads the menu once. After that it polls for orders every 3 seconds. Each HTTP request times out after 5 seconds.

## Controls

### Global

| Key   | Action               |
| ----- | -------------------- |
| `1`   | Order screen         |
| `2`   | Board screen         |
| `q`   | Quit                 |

### Order screen

The screen has three panes: **Menu**, **Customize** and **Your order**. `Tab` moves between them.

| Pane       | Key                  | Action                                     |
| ---------- | -------------------- | ------------------------------------------ |
| Menu       | `↑` / `↓`            | Select drink                               |
|            | `Enter` / `→`        | Customize the selected drink               |
| Customize  | `↑` / `↓`            | Move between fields                        |
|            | `←` / `→`            | Change milk or temperature, or move the add-on cursor |
|            | `Space` / `Enter`    | Toggle add-on, or **Add to order**         |
|            | `Esc`                | Back to menu                               |
| Your order | `↑` / `↓`            | Move between items, name field, and send   |
|            | `x` / `Delete`       | Remove item                                |
|            | type / `Backspace`   | Edit the customer name (max 16 chars; leave it blank for "Walk-in") |
|            | `Enter`              | **Send to bar**                            |
|            | `Esc`                | Back to menu                               |

Each drink in the cart becomes a separate order. After the order is sent, press `Enter` to start a new order or `2` to follow it on the board.

### Board screen

| Key               | Action                          |
| ----------------- | ------------------------------- |
| `←` / `→`         | Change column                   |
| `↑` / `↓`         | Select order                    |
| `Enter` / `Space` | Advance order to next status    |
| `s`               | Refresh now                     |

## Project layout

```
src/
  main.rs    # Entry point: sets up the client, event loop and terminal
  api.rs     # Blocking FeathersJS REST client (ureq) and data models
  app.rs     # Application state (tabs, cart, board selection)
  event.rs   # Input/tick thread and background menu/orders poller
  update.rs  # Key handling and event → state updates
  ui.rs      # Ratatui rendering for both screens and the "sent" modal
  tui.rs     # Terminal setup/teardown (raw mode, alternate screen, panic hook)
```

The app follows a simple Elm-style loop: `EventHandler` sends events (key presses, ticks, API results) through a channel, `update` changes `App`, and `ui::render` draws it. Network calls run on background threads, so the UI stays responsive.

## Development

```bash
cargo build --release   # Build the binary to target/release/buppers-terminal-gui
cargo test              # Run unit tests
cargo clippy            # Lint
cargo fmt               # Format
```
