use chrono::{DateTime, Utc};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::api::{Order, OrderStatus};
use crate::app::{App, CustomRow, Tab, Zone};

const TEAL: Color = Color::Rgb(0x3f, 0xbc, 0xae);
const DIM: Color = Color::Rgb(0x82, 0x90, 0x8c);
const LINE: Color = Color::Rgb(0x2f, 0x41, 0x3e);
const CREAM: Color = Color::Rgb(0xf8, 0xf7, 0xf2);
const RED: Color = Color::Rgb(0xe8, 0x74, 0x6a);
const AMBER: Color = Color::Rgb(0xe6, 0xb0, 0x66);
const GREEN: Color = Color::Rgb(0x6c, 0xc4, 0x9a);
const HEAD: Color = Color::Rgb(0x06, 0x46, 0x3f);

fn status_color(status: OrderStatus) -> Color {
    match status {
        OrderStatus::Ordered => RED,
        OrderStatus::Brewing => AMBER,
        OrderStatus::ReadyForPickup => GREEN,
        OrderStatus::Complete => DIM,
    }
}

fn ago(created_at: &str) -> String {
    let Ok(ts) = DateTime::parse_from_rfc3339(created_at) else {
        return String::new();
    };
    let mins = (Utc::now() - ts.with_timezone(&Utc)).num_minutes();
    if mins < 1 {
        "now".into()
    } else if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h", mins / 60)
    }
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    render_header(app, frame, chunks[0]);
    match app.tab {
        Tab::Order => render_order_tab(app, frame, chunks[1]),
        Tab::Board => render_board_tab(app, frame, chunks[1]),
    }
    render_status_line(app, frame, chunks[2]);
    render_footer(app, frame, chunks[3]);

    if let Some(placed) = app.placed.clone() {
        render_placed_modal(&placed, frame, area);
    }
}

fn render_status_line(app: &App, frame: &mut Frame, area: Rect) {
    let line = if !app.message.is_empty() {
        Line::from(Span::styled(app.message.clone(), Style::default().fg(GREEN)))
    } else if app.tab == Tab::Board {
        let status = OrderStatus::all()[app.board_col];
        match app.orders_in(status).get(app.board_idx) {
            Some(o) => {
                let mut spans = vec![Span::styled(
                    format!("#{} {} \u{b7} placed {} ago", o.order_id, o.name, ago(&o.created_at)),
                    Style::default().fg(DIM),
                )];
                if let Some(action) = o.status.action_label() {
                    spans.push(Span::styled(format!("   {action} -> "), Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD)));
                }
                Line::from(spans)
            }
            None => message_or_default(app),
        }
    } else {
        message_or_default(app)
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let new_count = app.orders.iter().filter(|o| o.status == OrderStatus::Ordered).count();
    let order_tab = if app.tab == Tab::Order {
        Span::styled(" 1 Order ", Style::default().bg(CREAM).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" 1 Order ", Style::default().bg(HEAD).fg(CREAM))
    };
    let board_label = if new_count > 0 {
        format!(" 2 Board \u{25cf}{new_count} ")
    } else {
        " 2 Board ".to_string()
    };
    let board_tab = if app.tab == Tab::Board {
        Span::styled(board_label, Style::default().bg(CREAM).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(board_label, Style::default().bg(HEAD).fg(RED).add_modifier(Modifier::BOLD))
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(14)])
        .split(area);

    let left = Line::from(vec![
        Span::styled(" BUPPERS", Style::default().bg(HEAD).fg(CREAM).add_modifier(Modifier::BOLD)),
        Span::styled(" \u{b7} bar terminal  ", Style::default().bg(HEAD).fg(Color::Rgb(0xbc, 0xd9, 0xd3))),
        order_tab,
        Span::styled(" ", Style::default().bg(HEAD)),
        board_tab,
    ]);
    frame.render_widget(Paragraph::new(left).style(Style::default().bg(HEAD)), cols[0]);

    let clock = chrono::Local::now().format("%H:%M").to_string();
    let right = Line::from(Span::styled(format!("{clock} \u{25cf} live "), Style::default().bg(HEAD).fg(GREEN)));
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right).style(Style::default().bg(HEAD)), cols[1]);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let hints: Vec<(&str, &str)> = if app.placed.is_some() {
        vec![("Enter", "new order"), ("2", "track on board")]
    } else if app.tab == Tab::Board {
        vec![("<- ->", "column"), ("up/dn", "order"), ("Enter", "advance"), ("s", "refresh"), ("1-2", "screens")]
    } else {
        match app.zone {
            Zone::Custom => vec![("up/dn", "field"), ("<- ->", "change"), ("space", "toggle"), ("Enter", "add"), ("Esc", "back")],
            Zone::Cart => vec![("up/dn", "move"), ("x", "remove"), ("Enter", "send"), ("Esc", "back")],
            Zone::Menu => vec![("up/dn", "move"), ("Enter", "customize"), ("Tab", "your order"), ("1-2", "screens")],
        }
    };
    let mut spans = vec![Span::raw(" ")];
    for (key, label) in hints {
        spans.push(Span::styled(format!(" {key} "), Style::default().bg(LINE).fg(CREAM).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {label}   "), Style::default().fg(DIM)));
    }
    spans.push(Span::styled(" q ", Style::default().bg(LINE).fg(CREAM).add_modifier(Modifier::BOLD)));
    spans.push(Span::styled(" quit", Style::default().fg(DIM)));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn message_or_default(app: &App) -> Line<'static> {
    if !app.message.is_empty() {
        return Line::from(Span::styled(app.message.clone(), Style::default().fg(GREEN)));
    }
    if app.loading {
        return Line::from(Span::styled("Loading menu...", Style::default().fg(DIM)));
    }
    if let Some(err) = &app.menu_error {
        return Line::from(Span::styled(format!("menu error: {err}"), Style::default().fg(RED)));
    }
    Line::from(Span::styled("", Style::default()))
}

fn render_order_tab(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(cols[1]);

    render_menu(app, frame, cols[0]);
    render_customize(app, frame, rows[0]);
    render_cart(app, frame, rows[1]);
}

fn render_menu(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.zone == Zone::Menu;
    let items: Vec<ListItem> = app
        .drinks
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let selected = i == app.menu_index;
            let marker = if selected && focused { "\u{25b8}" } else { " " };
            let base = if selected { Style::default().bg(TEAL).fg(Color::Black) } else { Style::default().fg(CREAM) };
            let mut name_line = vec![Span::styled(format!("{marker} "), Style::default().fg(TEAL).add_modifier(Modifier::BOLD)), Span::styled(d.name.clone(), base.add_modifier(Modifier::BOLD))];
            if d.featured {
                name_line.push(Span::styled("  NEW", Style::default().fg(RED).add_modifier(Modifier::BOLD)));
            }
            let desc = d.description.clone().unwrap_or_default();
            ListItem::new(vec![Line::from(name_line), Line::from(Span::styled(format!("  {desc}"), Style::default().fg(DIM)))])
        })
        .collect();
    let block = Block::default()
        .title(" Menu ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { TEAL } else { LINE }))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let has_bean = app.current_bean().is_some();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(if has_bean { 1 } else { 0 })])
        .split(inner);

    let mut state = ListState::default().with_selected(Some(app.menu_index));
    frame.render_stateful_widget(List::new(items), rows[0], &mut state);

    if let Some(bean) = app.current_bean() {
        let roaster = bean.roaster.clone().unwrap_or_default();
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!("Bean \u{b7} {} \u{b7} {}", bean.name, roaster), Style::default().fg(DIM)))),
            rows[1],
        );
    }
}

fn render_customize(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.zone == Zone::Custom;
    let block = Block::default()
        .title(" Customize ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { TEAL } else { LINE }))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(drink) = app.selected_drink() else {
        frame.render_widget(Paragraph::new("No drinks loaded."), inner);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut title = vec![Span::styled(drink.name.clone(), Style::default().fg(CREAM).add_modifier(Modifier::BOLD))];
    if drink.featured {
        title.push(Span::styled("  SEASONAL", Style::default().bg(RED).fg(Color::Black).add_modifier(Modifier::BOLD)));
    }
    lines.push(Line::from(title));
    if let Some(desc) = &drink.description {
        lines.push(Line::from(Span::styled(desc.clone(), Style::default().fg(DIM))));
    }
    lines.push(Line::from(""));

    let rows = app.custom_rows();
    for (ri, row) in rows.iter().enumerate() {
        let on = focused && app.custom_row == ri;
        let marker = Span::styled(if on { "\u{25b8} " } else { "  " }, Style::default().fg(TEAL).add_modifier(Modifier::BOLD));
        let label_style = if on { Style::default().fg(CREAM).add_modifier(Modifier::BOLD) } else { Style::default().fg(DIM) };
        match row {
            CustomRow::Milk => {
                let mut spans = vec![marker, Span::styled("Milk    ", label_style)];
                spans.push(radio(app.milk_index == 0, "No milk", on));
                for (i, m) in app.milks.iter().enumerate() {
                    spans.push(Span::raw("  "));
                    spans.push(radio(app.milk_index == i + 1, &m.name, on));
                }
                lines.push(Line::from(spans));
            }
            CustomRow::Temp => {
                let mut spans = vec![marker, Span::styled("Temp    ", label_style)];
                spans.push(radio(app.is_hot, "Hot", on));
                spans.push(Span::raw("  "));
                spans.push(radio(!app.is_hot, "Iced", on));
                lines.push(Line::from(spans));
            }
            CustomRow::Addons => {
                let mut spans = vec![marker, Span::styled("Add-ons ", label_style)];
                for (i, a) in app.addons.iter().enumerate() {
                    let checked = app.selected_addons.contains(&a.addon_id);
                    let cur = on && app.addon_cursor == i;
                    let text = format!("[{}] {}", if checked { "x" } else { " " }, a.name);
                    let style = if cur { Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD) } else if checked { Style::default().fg(TEAL).add_modifier(Modifier::BOLD) } else { Style::default().fg(CREAM) };
                    spans.push(Span::styled(text, style));
                    spans.push(Span::raw("  "));
                }
                lines.push(Line::from(spans));
            }
            CustomRow::Add => {
                let style = if on { Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEAL).add_modifier(Modifier::BOLD) };
                lines.push(Line::from(vec![marker, Span::styled(" Add to order Enter ", style)]));
            }
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn radio(selected: bool, label: &str, on: bool) -> Span<'static> {
    let text = format!("{} {}", if selected { "\u{25cf}" } else { "\u{25cb}" }, label);
    let style = if selected {
        if on { Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEAL).add_modifier(Modifier::BOLD) }
    } else {
        Style::default().fg(CREAM)
    };
    Span::styled(text, style)
}

fn render_cart(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.zone == Zone::Cart;
    let n = app.cart.len();
    let title = format!(" Your order \u{b7} {} {} ", n, if n == 1 { "drink" } else { "drinks" });
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { TEAL } else { LINE }))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if app.cart.is_empty() {
        lines.push(Line::from(Span::styled("Nothing in the cup yet. Pick a drink.", Style::default().fg(DIM))));
    }
    for (i, item) in app.cart.iter().enumerate() {
        let on = focused && app.cart_index == i;
        let marker = if on { "\u{25b8} " } else { "  " };
        let base = if on { Style::default().bg(TEAL).fg(Color::Black) } else { Style::default().fg(CREAM) };
        let mut detail_parts: Vec<String> = Vec::new();
        detail_parts.push(if item.is_hot { "Hot".into() } else { "Iced".into() });
        if let Some(m) = &item.milk_label {
            detail_parts.push(m.clone());
        }
        for a in &item.addon_labels {
            detail_parts.push(format!("+{a}"));
        }
        let detail = detail_parts.join(" \u{b7} ");
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(TEAL).add_modifier(Modifier::BOLD)),
            Span::styled(item.drink_name.clone(), base.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {detail}"), Style::default().fg(DIM)),
        ]));
    }
    if !app.cart.is_empty() {
        lines.push(Line::from(""));
        let on_name = focused && app.cart_index == app.cart.len();
        let on_send = focused && app.cart_index == app.cart.len() + 1;
        let mut name_spans = vec![
            Span::styled(if on_name || on_send { "\u{25b8} " } else { "  " }, Style::default().fg(TEAL).add_modifier(Modifier::BOLD)),
            Span::styled("Name  ", Style::default().fg(DIM)),
        ];
        if on_name {
            let field_style = Style::default().bg(CREAM).fg(Color::Black);
            name_spans.push(Span::styled(format!(" {}", app.customer_name), field_style));
            let blink_on = (app.tick_count / 2).is_multiple_of(2);
            let caret_style = if blink_on { field_style.add_modifier(Modifier::REVERSED) } else { field_style };
            name_spans.push(Span::styled(" ", caret_style));
            if app.customer_name.is_empty() {
                name_spans.push(Span::styled("  (Walk-in if left blank)", Style::default().fg(DIM)));
            }
        } else {
            let name_text = if app.customer_name.is_empty() { "Walk-in".to_string() } else { app.customer_name.clone() };
            name_spans.push(Span::styled(format!(" {name_text} "), Style::default().fg(DIM)));
        }
        lines.push(Line::from(name_spans));
        let send_style = if on_send { Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEAL).add_modifier(Modifier::BOLD) };
        lines.push(Line::from(Span::styled("   Send to bar Enter ", send_style)));
    }
    if app.submitting {
        lines.push(Line::from(Span::styled("Sending...", Style::default().fg(AMBER))));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_board_tab(app: &App, frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(area);

    let today = Utc::now().date_naive();
    let is_today = |o: &&Order| {
        DateTime::parse_from_rfc3339(&o.created_at)
            .map(|t| t.with_timezone(&Utc).date_naive() == today)
            .unwrap_or(false)
    };
    let in_progress = app.orders.iter().filter(|o| matches!(o.status, OrderStatus::Ordered | OrderStatus::Brewing)).count();
    let ready = app.orders.iter().filter(|o| o.status == OrderStatus::ReadyForPickup).count();
    let done_today = app.orders.iter().filter(|o| o.status == OrderStatus::Complete).filter(is_today).count();
    let today_count = app.orders.iter().filter(is_today).count();
    let stats = Line::from(vec![
        Span::styled(format!(" Today {today_count}   "), Style::default().fg(CREAM)),
        Span::styled(format!("In progress {in_progress}   "), Style::default().fg(AMBER)),
        Span::styled(format!("Ready {ready}   "), Style::default().fg(GREEN)),
        Span::styled(format!("Done today {done_today}"), Style::default().fg(TEAL)),
    ]);
    frame.render_widget(Paragraph::new(stats), rows[0]);

    let statuses = OrderStatus::all();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25); 4])
        .split(rows[1]);

    for (ci, status) in statuses.iter().enumerate() {
        let focused = app.board_col == ci;
        let list = app.orders_in(*status);
        let block = Block::default()
            .title(format!(" {} {} ", status.label(), list.len()))
            .title_style(Style::default().fg(status_color(*status)).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused { TEAL } else { LINE }));
        let inner = block.inner(cols[ci]);
        frame.render_widget(block, cols[ci]);

        if list.is_empty() {
            frame.render_widget(Paragraph::new(Span::styled("\u{2014}", Style::default().fg(DIM))), inner);
            continue;
        }
        let items: Vec<ListItem> = list
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let selected = focused && app.board_idx == i;
                let base = if selected { Style::default().bg(TEAL).fg(Color::Black) } else { Style::default().fg(CREAM) };
                let header = Line::from(vec![
                    Span::styled(format!("#{} {}", o.order_id, o.name), base.add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  {}", ago(&o.created_at)), Style::default().fg(DIM)),
                ]);
                let detail = Line::from(Span::styled(
                    format!("  {}{}", if o.is_hot { "" } else { "iced " }, drink_summary(app, o)),
                    if selected { Style::default().fg(Color::Black).bg(TEAL) } else { Style::default().fg(DIM) },
                ));
                ListItem::new(vec![header, detail])
            })
            .collect();
        let mut state = ListState::default();
        if focused {
            state = state.with_selected(Some(app.board_idx));
        }
        frame.render_stateful_widget(List::new(items), inner, &mut state);
    }
}

fn drink_summary(app: &App, o: &Order) -> String {
    let mut parts = Vec::new();
    match app.drinks.iter().find(|d| d.drink_id == o.drink_id) {
        Some(drink) => parts.push(drink.name.clone()),
        None => parts.push(format!("Drink #{}", o.drink_id)),
    }
    if let Some(milk_id) = o.milk_id
        && let Some(milk) = app.milks.iter().find(|m| m.milk_id == milk_id)
    {
        parts.push(milk.name.clone());
    }
    if !o.addons.is_empty() {
        parts.push(o.addons.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "));
    }
    parts.join(" \u{b7} ")
}

fn render_placed_modal(placed: &crate::app::Placed, frame: &mut Frame, area: Rect) {
    let width = 52.min(area.width.saturating_sub(4)).max(20);
    let height = (8 + placed.items.len() as u16).min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect { x, y, width, height };
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Sent to the bar ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .style(Style::default().bg(Color::Rgb(0x13, 0x22, 0x20)));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let mut lines = vec![
        Line::from(Span::styled(format!("\u{2713} Order in, {}!", placed.name), Style::default().fg(GREEN).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("Buppers is on it. We'll call your name.", Style::default().fg(DIM))),
        Line::from(""),
        Line::from(Span::styled(
            format!("Order number{}#{}", " ".repeat(10), placed.order_ids.first().copied().unwrap_or_default()),
            Style::default().fg(TEAL),
        )),
    ];
    for item in &placed.items {
        lines.push(Line::from(item.drink_name.clone()));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter New order ", Style::default().bg(TEAL).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(" 2 Track on board ", Style::default().bg(LINE).fg(CREAM).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
