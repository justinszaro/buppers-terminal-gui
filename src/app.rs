use std::sync::mpsc::Sender;

use crate::api::{Addon, Bean, Client, Drink, Milk, Order, OrderStatus};
use crate::event::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Order,
    Board,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Menu,
    Custom,
    Cart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomRow {
    Milk,
    Temp,
    Addons,
    Add,
}

#[derive(Debug, Clone)]
pub struct CartItem {
    pub drink_id: i64,
    pub drink_name: String,
    pub milk_label: Option<String>,
    pub milk_id: Option<i64>,
    pub is_hot: bool,
    pub addon_ids: Vec<i64>,
    pub addon_labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Placed {
    pub name: String,
    pub order_ids: Vec<i64>,
    pub items: Vec<CartItem>,
}

pub struct App {
    pub should_quit: bool,
    pub client: Client,
    pub event_tx: Sender<Event>,

    pub tab: Tab,
    pub loading: bool,
    pub menu_error: Option<String>,
    pub message: String,
    pub tick_count: u64,

    // Menu data from the API.
    pub drinks: Vec<Drink>,
    pub beans: Vec<Bean>,
    pub milks: Vec<Milk>,
    pub addons: Vec<Addon>,

    // Order tab state.
    pub zone: Zone,
    pub menu_index: usize,
    pub custom_row: usize,
    pub milk_index: usize,
    pub is_hot: bool,
    pub addon_cursor: usize,
    pub selected_addons: Vec<i64>,
    pub cart: Vec<CartItem>,
    pub cart_index: usize,
    pub customer_name: String,
    pub placed: Option<Placed>,
    pub submitting: bool,

    // Board tab state.
    pub orders: Vec<Order>,
    pub board_col: usize,
    pub board_idx: usize,
}

impl App {
    pub fn new(client: Client, event_tx: Sender<Event>) -> Self {
        Self {
            should_quit: false,
            client,
            event_tx,
            tab: Tab::Order,
            loading: true,
            menu_error: None,
            message: String::new(),
            tick_count: 0,
            drinks: Vec::new(),
            beans: Vec::new(),
            milks: Vec::new(),
            addons: Vec::new(),
            zone: Zone::Menu,
            menu_index: 0,
            custom_row: 0,
            milk_index: 0,
            is_hot: true,
            addon_cursor: 0,
            selected_addons: Vec::new(),
            cart: Vec::new(),
            cart_index: 0,
            customer_name: String::new(),
            placed: None,
            submitting: false,
            orders: Vec::new(),
            board_col: 0,
            board_idx: 0,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn selected_drink(&self) -> Option<&Drink> {
        self.drinks.get(self.menu_index)
    }

    pub fn current_bean(&self) -> Option<&Bean> {
        self.beans.iter().find(|b| b.on_deck).or(self.beans.first())
    }

    pub fn custom_rows(&self) -> Vec<CustomRow> {
        let mut rows = vec![CustomRow::Milk];
        if self.selected_drink().map(|d| d.temp.is_none()).unwrap_or(true) {
            rows.push(CustomRow::Temp);
        }
        if !self.addons.is_empty() {
            rows.push(CustomRow::Addons);
        }
        rows.push(CustomRow::Add);
        rows
    }

    pub fn reset_custom_options(&mut self) {
        self.custom_row = 0;
        self.milk_index = 0;
        self.addon_cursor = 0;
        self.selected_addons.clear();
        self.is_hot = !matches!(self.selected_drink().and_then(|d| d.temp.as_deref()), Some("cold"));
    }

    pub fn milk_label(&self) -> Option<String> {
        if self.milk_index == 0 {
            None
        } else {
            self.milks.get(self.milk_index - 1).map(|m| m.name.clone())
        }
    }

    pub fn milk_id(&self) -> Option<i64> {
        if self.milk_index == 0 {
            None
        } else {
            self.milks.get(self.milk_index - 1).map(|m| m.milk_id)
        }
    }

    pub fn toggle_addon(&mut self, idx: usize) {
        if let Some(addon) = self.addons.get(idx) {
            if let Some(pos) = self.selected_addons.iter().position(|id| *id == addon.addon_id) {
                self.selected_addons.remove(pos);
            } else {
                self.selected_addons.push(addon.addon_id);
            }
        }
    }

    pub fn add_to_cart(&mut self) {
        let Some(drink) = self.selected_drink().cloned() else {
            return;
        };
        let addon_labels = self
            .addons
            .iter()
            .filter(|a| self.selected_addons.contains(&a.addon_id))
            .map(|a| a.name.clone())
            .collect();
        self.cart.push(CartItem {
            drink_id: drink.drink_id,
            drink_name: drink.name.clone(),
            milk_label: self.milk_label(),
            milk_id: self.milk_id(),
            is_hot: self.is_hot,
            addon_ids: self.selected_addons.clone(),
            addon_labels,
        });
        self.message = format!("Added {} to your order. Tab to review.", drink.name);
        self.zone = Zone::Menu;
        self.reset_custom_options();
    }

    pub fn remove_cart_item(&mut self, idx: usize) {
        if idx < self.cart.len() {
            self.cart.remove(idx);
        }
        if self.cart.is_empty() {
            self.zone = Zone::Menu;
            self.cart_index = 0;
        } else {
            self.cart_index = self.cart_index.min(self.cart.len() + 1);
        }
    }

    pub fn orders_in(&self, status: OrderStatus) -> Vec<&Order> {
        let mut list: Vec<&Order> = self.orders.iter().filter(|o| o.status == status).collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub fn clear_placed(&mut self, go_to_board: bool) {
        self.placed = None;
        self.cart.clear();
        self.customer_name.clear();
        self.zone = Zone::Menu;
        self.cart_index = 0;
        if go_to_board {
            self.tab = Tab::Board;
            self.board_col = 0;
            self.board_idx = 0;
        }
    }
}
