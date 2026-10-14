use std::thread;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::{NewOrder, OrderStatus};
use crate::app::{App, CustomRow, Tab, Zone};
use crate::event::Event;

pub fn update(app: &mut App, event: Event) {
    match event {
        Event::Tick => app.tick_count = app.tick_count.wrapping_add(1),
        Event::Mouse(_) | Event::Resize(_, _) => {}
        Event::Key(key) => handle_key(app, key),
        Event::MenuLoaded(Ok(menu)) => {
            app.drinks = menu.drinks;
            app.beans = menu.beans;
            app.milks = menu.milks;
            app.addons = menu.addons;
            app.loading = false;
            app.reset_custom_options();
        }
        Event::MenuLoaded(Err(err)) => {
            app.loading = false;
            app.menu_error = Some(err);
        }
        Event::OrdersLoaded(Ok(orders)) => {
            app.orders = orders;
            let len = app.orders_in(board_status(app)).len();
            app.board_idx = app.board_idx.min(len.saturating_sub(1));
        }
        Event::OrdersLoaded(Err(err)) => {
            app.message = format!("orders refresh failed: {err}");
        }
        Event::OrderSubmitted(Ok(created)) => {
            app.submitting = false;
            if let Some(first) = created.first() {
                app.placed = Some(crate::app::Placed {
                    name: first.name.clone(),
                    order_ids: created.iter().map(|o| o.order_id).collect(),
                    items: app.cart.clone(),
                });
            }
        }
        Event::OrderSubmitted(Err(err)) => {
            app.submitting = false;
            app.message = format!("send failed: {err}");
        }
        Event::StatusChanged(Ok(order)) => {
            app.submitting = false;
            app.message = format!("#{} {} -> {}", order.order_id, order.name, order.status.label());
            if let Some(existing) = app.orders.iter_mut().find(|o| o.order_id == order.order_id) {
                *existing = order;
            }
        }
        Event::StatusChanged(Err(err)) => {
            app.submitting = false;
            app.message = format!("update failed: {err}");
        }
    }
}

fn board_status(app: &App) -> OrderStatus {
    OrderStatus::all()[app.board_col]
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return;
    }
    app.message.clear();

    if app.placed.is_some() {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => app.clear_placed(false),
            KeyCode::Char('2') => app.clear_placed(true),
            _ => {}
        }
        return;
    }

    let typing_name = app.tab == Tab::Order && app.zone == Zone::Cart && app.cart_index == app.cart.len();
    if !typing_name {
        match key.code {
            KeyCode::Char('1') => {
                app.tab = Tab::Order;
                return;
            }
            KeyCode::Char('2') => {
                app.tab = Tab::Board;
                return;
            }
            KeyCode::Char('q') => {
                app.quit();
                return;
            }
            _ => {}
        }
    }

    match app.tab {
        Tab::Order => handle_order_key(app, key.code),
        Tab::Board => handle_board_key(app, key.code),
    }
}

fn handle_order_key(app: &mut App, key: KeyCode) {
    match app.zone {
        Zone::Menu => {
            let len = app.drinks.len();
            if len == 0 {
                return;
            }
            match key {
                KeyCode::Up => {
                    app.menu_index = (app.menu_index + len - 1) % len;
                    app.reset_custom_options();
                }
                KeyCode::Down => {
                    app.menu_index = (app.menu_index + 1) % len;
                    app.reset_custom_options();
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char(' ') => {
                    app.zone = Zone::Custom;
                }
                KeyCode::Tab => {
                    if !app.cart.is_empty() {
                        app.zone = Zone::Cart;
                        app.cart_index = app.cart.len();
                    } else {
                        app.zone = Zone::Custom;
                    }
                }
                _ => {}
            }
        }
        Zone::Custom => {
            let rows = app.custom_rows();
            let row = rows.get(app.custom_row).cloned();
            match key {
                KeyCode::Esc => app.zone = Zone::Menu,
                KeyCode::Tab => {
                    app.zone = if !app.cart.is_empty() { Zone::Cart } else { Zone::Menu };
                    if app.zone == Zone::Cart {
                        app.cart_index = app.cart.len();
                    }
                }
                KeyCode::Up => app.custom_row = app.custom_row.saturating_sub(1),
                KeyCode::Down => app.custom_row = (app.custom_row + 1).min(rows.len() - 1),
                KeyCode::Left | KeyCode::Right => {
                    let dir: i64 = if key == KeyCode::Left { -1 } else { 1 };
                    match row {
                        Some(CustomRow::Milk) => {
                            let max = app.milks.len() as i64;
                            app.milk_index = (app.milk_index as i64 + dir).clamp(0, max) as usize;
                        }
                        Some(CustomRow::Temp) => app.is_hot = !app.is_hot,
                        Some(CustomRow::Addons) => {
                            let max = app.addons.len().saturating_sub(1) as i64;
                            app.addon_cursor = (app.addon_cursor as i64 + dir).clamp(0, max) as usize;
                        }
                        _ => {}
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => match row {
                    Some(CustomRow::Addons) => {
                        let idx = app.addon_cursor;
                        app.toggle_addon(idx);
                    }
                    Some(CustomRow::Add) => app.add_to_cart(),
                    _ => app.custom_row = (app.custom_row + 1).min(rows.len() - 1),
                },
                _ => {}
            }
        }
        Zone::Cart => handle_cart_key(app, key),
    }
}

fn handle_cart_key(app: &mut App, key: KeyCode) {
    let n = app.cart.len();
    if app.cart_index == n {
        match key {
            KeyCode::Backspace => {
                app.customer_name.pop();
                return;
            }
            KeyCode::Char(c) if app.customer_name.len() < 16 && (c.is_alphanumeric() || " .'-".contains(c)) => {
                app.customer_name.push(c);
                return;
            }
            _ => {}
        }
    }
    match key {
        KeyCode::Esc | KeyCode::Tab => app.zone = Zone::Menu,
        KeyCode::Up => app.cart_index = app.cart_index.saturating_sub(1),
        KeyCode::Down => app.cart_index = (app.cart_index + 1).min(n + 1),
        KeyCode::Char('x') | KeyCode::Delete | KeyCode::Backspace if app.cart_index < n => {
            app.remove_cart_item(app.cart_index);
        }
        KeyCode::Enter if app.cart_index < n => app.cart_index = n,
        KeyCode::Enter => submit_order(app),
        _ => {}
    }
}

fn handle_board_key(app: &mut App, key: KeyCode) {
    let statuses = OrderStatus::all();
    let col_len = |app: &App, col: usize| app.orders_in(statuses[col]).len();
    match key {
        KeyCode::Left => {
            app.board_col = app.board_col.saturating_sub(1);
            app.board_idx = app.board_idx.min(col_len(app, app.board_col).saturating_sub(1));
        }
        KeyCode::Right => {
            app.board_col = (app.board_col + 1).min(3);
            app.board_idx = app.board_idx.min(col_len(app, app.board_col).saturating_sub(1));
        }
        KeyCode::Up => app.board_idx = app.board_idx.saturating_sub(1),
        KeyCode::Down => {
            app.board_idx = (app.board_idx + 1).min(col_len(app, app.board_col).saturating_sub(1));
        }
        KeyCode::Enter | KeyCode::Char(' ') => advance_selected(app),
        KeyCode::Char('s') => {
            // force an immediate refresh instead of waiting for the poller
            request_orders_refresh(app);
        }
        _ => {}
    }
}

fn advance_selected(app: &mut App) {
    if app.submitting {
        return;
    }
    let status = board_status(app);
    let Some(order) = app.orders_in(status).get(app.board_idx).cloned() else {
        return;
    };
    let Some(next) = order.status.next() else {
        app.message = format!("#{} is already picked up.", order.order_id);
        return;
    };
    let order_id = order.order_id;
    app.submitting = true;
    let client = app.client.clone();
    let tx = app.event_tx.clone();
    thread::spawn(move || {
        let result = client.advance_order(order_id, next).map_err(|e| e.to_string());
        let _ = tx.send(Event::StatusChanged(result));
    });
}

fn request_orders_refresh(app: &App) {
    let client = app.client.clone();
    let tx = app.event_tx.clone();
    thread::spawn(move || {
        let result = client.fetch_orders().map_err(|e| e.to_string());
        let _ = tx.send(Event::OrdersLoaded(result));
    });
}

fn submit_order(app: &mut App) {
    if app.cart.is_empty() || app.submitting {
        return;
    }
    app.submitting = true;
    let name = if app.customer_name.trim().is_empty() {
        "Walk-in".to_string()
    } else {
        app.customer_name.trim().to_string()
    };
    let items = app.cart.clone();
    let client = app.client.clone();
    let tx = app.event_tx.clone();
    thread::spawn(move || {
        let mut created = Vec::new();
        let mut error = None;
        for item in &items {
            let payload = NewOrder {
                drink_id: item.drink_id,
                milk_id: item.milk_id,
                name: name.clone(),
                is_hot: item.is_hot,
                addon_ids: item.addon_ids.clone(),
            };
            match client.create_order(&payload) {
                Ok(order) => created.push(order),
                Err(err) => {
                    error = Some(err.to_string());
                    break;
                }
            }
        }
        let result = match error {
            Some(err) => Err(err),
            None => Ok(created),
        };
        let _ = tx.send(Event::OrderSubmitted(result));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Client;
    use crate::app::CartItem;

    #[test]
    fn name_field_accepts_typing() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut app = App::new(Client::new(), tx);
        app.cart.push(CartItem {
            drink_id: 1,
            drink_name: "Latte".into(),
            milk_label: None,
            milk_id: None,
            is_hot: true,
            addon_ids: vec![],
            addon_labels: vec![],
        });
        app.tab = Tab::Order;
        app.zone = Zone::Cart;
        app.cart_index = app.cart.len();

        let key = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE);
        update(&mut app, Event::Key(key));
        assert_eq!(app.customer_name, "J");
    }
}

#[cfg(test)]
mod cart_editability_tests {
    use super::*;
    use crate::api::Client;
    use crate::app::CartItem;

    fn sample_item(name: &str) -> CartItem {
        CartItem {
            drink_id: 1,
            drink_name: name.into(),
            milk_label: None,
            milk_id: None,
            is_hot: true,
            addon_ids: vec![],
            addon_labels: vec![],
        }
    }

    #[test]
    fn backspace_edits_name() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut app = App::new(Client::new(), tx);
        app.cart.push(sample_item("Latte"));
        app.zone = Zone::Cart;
        app.cart_index = app.cart.len();
        app.customer_name = "Jo".into();
        update(&mut app, Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)));
        assert_eq!(app.customer_name, "J");
    }

    #[test]
    fn can_remove_item_then_add_another_keeping_name() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut app = App::new(Client::new(), tx);
        app.cart.push(sample_item("Latte"));
        app.cart.push(sample_item("Mocha"));
        app.zone = Zone::Cart;
        app.cart_index = 0;
        app.customer_name = "Priya".into();

        // remove the first item while reviewing the cart
        update(&mut app, Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)));
        assert_eq!(app.cart.len(), 1);
        assert_eq!(app.cart[0].drink_name, "Mocha");
        assert_eq!(app.customer_name, "Priya", "removing an item must not clear the typed name");

        // go back to the menu to add another drink, cart/name must survive
        update(&mut app, Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        assert_eq!(app.zone, Zone::Menu);
        app.cart.push(sample_item("Cortado"));
        assert_eq!(app.cart.len(), 2);
        assert_eq!(app.customer_name, "Priya");
    }
}
