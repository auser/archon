use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Focus, Mode};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: content area + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(size);

    // Three-column layout
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(45),
            Constraint::Percentage(30),
        ])
        .split(outer[0]);

    draw_node_list(f, app, columns[0]);
    draw_center_panel(f, app, columns[1]);
    draw_detail_panel(f, app, columns[2]);
    draw_status_bar(f, app, outer[1]);

    // Overlays
    if let Some(ref popup) = app.action_output {
        draw_popup(f, popup, size);
    }

    if app.show_help {
        draw_help(f, size);
    }
}

fn draw_node_list(f: &mut Frame, app: &mut App, area: Rect) {
    let title = match &app.node_list.role_filter {
        Some(role) => format!(" Repos [{}] ", role),
        None => " Repos ".to_string(),
    };

    let border_style = if app.focus == Focus::List && app.edit_state.is_none() {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .node_list
        .items
        .iter()
        .map(|name| {
            let node = app.graph.find_node(name);
            let role_color = node
                .map(|n| App::role_color(&n.role))
                .unwrap_or(Color::White);

            let is_impacted = app.mode == Mode::Impact && app.impact_nodes.contains(name);
            let is_path_node = app
                .path_finder
                .result
                .as_ref()
                .map(|p| p.contains(name))
                .unwrap_or(false);

            let style = if is_impacted {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if is_path_node {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let role_tag = node
                .map(|n| format!("{}", n.role))
                .unwrap_or_default();
            let role_span = Span::styled(
                format!("[{}] ", &role_tag[..role_tag.len().min(3)]),
                Style::default().fg(role_color),
            );

            // Show dep toggle marker in edit mode
            let edit_marker = if let Some(ref edit) = app.edit_state {
                if edit.field_index == 2 && edit.depends_on.contains(name) {
                    Span::styled("[x] ", Style::default().fg(Color::Green))
                } else if edit.field_index == 2 {
                    Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
                } else {
                    Span::raw("")
                }
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                edit_marker,
                role_span,
                Span::styled(name.clone(), style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut app.node_list.state);

    // Search bar at bottom of list area
    if app.search.active || !app.search.query.is_empty() {
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(2),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let search_text = if app.search.active {
            format!("/{}_", app.search.query)
        } else {
            format!("/{}", app.search.query)
        };
        let search = Paragraph::new(search_text)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(search, search_area);
    }
}

fn draw_center_panel(f: &mut Frame, app: &App, area: Rect) {
    match app.mode {
        Mode::Normal => draw_ego_graph(f, app, area),
        Mode::Tree => draw_tree_view(f, app, area),
        Mode::PathSelect => draw_path_view(f, app, area),
        Mode::Impact => draw_ego_graph(f, app, area),
    }
}

fn draw_ego_graph(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.mode {
        Mode::Impact => " Graph [Impact] ",
        _ => " Graph ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let node = match app.selected_node() {
        Some(n) => n,
        None => {
            let msg = Paragraph::new("No node selected")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(msg, inner);
            return;
        }
    };

    // Build the ego-graph: deps above, selected in center, dependents below
    let deps = &node.depends_on;
    let dependents = &node.dependents;

    let mut lines: Vec<Line> = Vec::new();
    let w = inner.width as usize;

    // --- Dependencies (top tier) ---
    if !deps.is_empty() {
        let (dep_names, overflow) = clamp_names(deps, 6);
        let boxes = render_box_row(&dep_names, w, app, &node.name);
        lines.extend(boxes);

        // Connector lines from deps down to selected
        let connector = render_merge_down(dep_names.len(), w);
        lines.extend(connector);

        if overflow > 0 {
            let overflow_line = format!("{}+{} more", " ".repeat(w / 2 - 4), overflow);
            lines.push(Line::from(Span::styled(
                overflow_line,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // --- Selected node (center) ---
    lines.push(Line::raw(""));
    let selected_box = render_single_box(&node.name, w, true, App::role_color(&node.role));
    lines.extend(selected_box);
    lines.push(Line::raw(""));

    // --- Dependents (bottom tier) ---
    if !dependents.is_empty() {
        let (dep_names, overflow) = clamp_names(dependents, 6);

        // Connector lines from selected down to dependents
        let connector = render_merge_up(dep_names.len(), w);
        lines.extend(connector);

        let boxes = render_box_row(&dep_names, w, app, &node.name);
        lines.extend(boxes);

        if overflow > 0 {
            let overflow_line = format!("{}+{} more", " ".repeat(w / 2 - 4), overflow);
            lines.push(Line::from(Span::styled(
                overflow_line,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Center vertically
    let content_height = lines.len();
    let available = inner.height as usize;
    let padding = if available > content_height {
        (available - content_height) / 2
    } else {
        0
    };

    let mut padded: Vec<Line> = Vec::new();
    for _ in 0..padding {
        padded.push(Line::raw(""));
    }
    padded.extend(lines);

    let para = Paragraph::new(padded);
    f.render_widget(para, inner);
}

fn clamp_names(names: &[String], max: usize) -> (Vec<String>, usize) {
    if names.len() <= max {
        (names.to_vec(), 0)
    } else {
        (names[..max].to_vec(), names.len() - max)
    }
}

fn render_box_row<'a>(names: &[String], width: usize, app: &App, _center_name: &str) -> Vec<Line<'a>> {
    // Each box: ┌──name──┐ / │  name  │ / └────────┘
    let box_width = |name: &str| -> usize { name.len() + 4 };

    let total_width: usize = names.iter().map(|n| box_width(n)).sum::<usize>()
        + names.len().saturating_sub(1) * 2; // spacing between boxes

    let start_pad = if width > total_width {
        (width - total_width) / 2
    } else {
        0
    };

    let mut top = String::new();
    let mut mid = String::new();
    let mut bot = String::new();

    top.push_str(&" ".repeat(start_pad));
    mid.push_str(&" ".repeat(start_pad));
    bot.push_str(&" ".repeat(start_pad));

    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            top.push_str("  ");
            mid.push_str("  ");
            bot.push_str("  ");
        }

        let bw = name.len() + 2;
        top.push('\u{250c}'); // ┌
        top.push_str(&"\u{2500}".repeat(bw)); // ─
        top.push('\u{2510}'); // ┐

        mid.push('\u{2502}'); // │
        mid.push(' ');
        mid.push_str(name);
        mid.push(' ');
        mid.push('\u{2502}'); // │

        bot.push('\u{2514}'); // └
        bot.push_str(&"\u{2500}".repeat(bw)); // ─
        bot.push('\u{2518}'); // ┘
    }

    // Color the boxes based on role and impact
    let top_line = Line::from(Span::styled(top, Style::default().fg(Color::DarkGray)));
    let bot_line = Line::from(Span::styled(bot, Style::default().fg(Color::DarkGray)));

    // For the middle line, color each name by role
    let mut mid_spans: Vec<Span> = Vec::new();
    mid_spans.push(Span::raw(" ".repeat(start_pad)));

    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            mid_spans.push(Span::raw("  "));
        }

        let node = app.graph.find_node(name);
        let role_color = node.map(|n| App::role_color(&n.role)).unwrap_or(Color::White);

        let is_impacted = app.mode == Mode::Impact && app.impact_nodes.contains(name);
        let color = if is_impacted { Color::Red } else { role_color };

        mid_spans.push(Span::styled("\u{2502} ", Style::default().fg(Color::DarkGray)));
        mid_spans.push(Span::styled(name.clone(), Style::default().fg(color).add_modifier(Modifier::BOLD)));
        mid_spans.push(Span::styled(" \u{2502}", Style::default().fg(Color::DarkGray)));
    }

    let mid_line = Line::from(mid_spans);

    vec![top_line, mid_line, bot_line]
}

fn render_single_box(name: &str, width: usize, highlight: bool, color: Color) -> Vec<Line<'static>> {
    let bw = name.len() + 2;
    let total = bw + 2; // box chars
    let pad = if width > total {
        (width - total) / 2
    } else {
        0
    };
    let padding = " ".repeat(pad);

    let border_style = if highlight {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let name_style = if highlight {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    let top = format!("{}\u{250c}{}\u{2510}", padding, "\u{2500}".repeat(bw));
    let bot = format!("{}\u{2514}{}\u{2518}", padding, "\u{2500}".repeat(bw));

    vec![
        Line::from(Span::styled(top, border_style)),
        Line::from(vec![
            Span::styled(format!("{}\u{2502} ", padding), border_style),
            Span::styled(name.to_string(), name_style),
            Span::styled(" \u{2502}".to_string(), border_style),
        ]),
        Line::from(Span::styled(bot, border_style)),
    ]
}

fn render_merge_down(count: usize, width: usize) -> Vec<Line<'static>> {
    if count == 0 {
        return vec![];
    }
    let center = width / 2;
    let mut line = " ".repeat(width);
    if center < line.len() {
        line.replace_range(center..center + 1, "\u{2502}"); // │
    }
    vec![Line::from(Span::styled(
        line,
        Style::default().fg(Color::Green),
    ))]
}

fn render_merge_up(count: usize, width: usize) -> Vec<Line<'static>> {
    if count == 0 {
        return vec![];
    }
    let center = width / 2;
    let mut line = " ".repeat(width);
    if center < line.len() {
        line.replace_range(center..center + 1, "\u{2502}"); // │
    }
    vec![Line::from(Span::styled(
        line,
        Style::default().fg(Color::Green),
    ))]
}

fn draw_tree_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Tree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.tree_state.flat_rows.is_empty() {
        let msg = Paragraph::new("No tree to display")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .tree_state
        .flat_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let indent = "  ".repeat(row.depth);
            let prefix = if row.has_children {
                if row.is_expanded {
                    "\u{25bc} " // ▼
                } else {
                    "\u{25b6} " // ▶
                }
            } else {
                "  "
            };

            let node = app.graph.find_node(&row.name);
            let role_color = node
                .map(|n| App::role_color(&n.role))
                .unwrap_or(Color::White);

            let style = if i == app.tree_state.cursor {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(indent, Style::default().fg(Color::DarkGray)),
                Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(row.name.clone(), Style::default().fg(role_color)),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

fn draw_path_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Path Finder ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref from) = app.path_finder.from {
        lines.push(Line::from(vec![
            Span::styled("From: ", Style::default().fg(Color::DarkGray)),
            Span::styled(from.clone(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
    }

    if let Some(ref to) = app.path_finder.to {
        lines.push(Line::from(vec![
            Span::styled("  To: ", Style::default().fg(Color::DarkGray)),
            Span::styled(to.clone(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "Select target node and press Enter",
            Style::default().fg(Color::Yellow),
        )));
    }

    lines.push(Line::raw(""));

    if let Some(ref path) = app.path_finder.result {
        lines.push(Line::from(Span::styled(
            format!("Path ({} hops):", path.len() - 1),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        for (i, name) in path.iter().enumerate() {
            if i > 0 {
                let arrow_pad = "  ".repeat(i);
                lines.push(Line::from(Span::styled(
                    format!("{}  \u{2193} depends on", arrow_pad),
                    Style::default().fg(Color::Green),
                )));
            }
            let node = app.graph.find_node(name);
            let role_color = node
                .map(|n| App::role_color(&n.role))
                .unwrap_or(Color::White);
            let desc = node
                .map(|n| format!(" ({})", n.description))
                .unwrap_or_default();
            let pad = "  ".repeat(i);
            lines.push(Line::from(vec![
                Span::raw(pad),
                Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(role_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else if app.path_finder.from.is_some() && app.path_finder.to.is_some() {
        lines.push(Line::from(Span::styled(
            "No path found.",
            Style::default().fg(Color::Red),
        )));
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    // Edit mode overrides the detail panel
    if let Some(ref edit) = app.edit_state {
        draw_edit_panel(f, edit, area);
        return;
    }

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let node = match app.selected_node() {
        Some(n) => n,
        None => {
            let msg = Paragraph::new("Select a node to view details")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(msg, inner);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Name
    lines.push(Line::from(Span::styled(
        &node.name,
        Style::default()
            .fg(App::role_color(&node.role))
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    // Description
    lines.push(Line::from(vec![
        Span::styled("desc  ", Style::default().fg(Color::DarkGray)),
        Span::raw(&node.description),
    ]));

    // Role
    lines.push(Line::from(vec![
        Span::styled("role  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", node.role),
            Style::default().fg(App::role_color(&node.role)),
        ),
    ]));

    lines.push(Line::raw(""));

    // Provides
    if !node.provides.is_empty() {
        lines.push(Line::from(Span::styled(
            "provides:",
            Style::default().fg(Color::DarkGray),
        )));
        for p in &node.provides {
            lines.push(Line::from(vec![
                Span::styled("  \u{2022} ", Style::default().fg(Color::Green)),
                Span::raw(p),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Dependencies
    lines.push(Line::from(Span::styled(
        format!("depends on ({}):", node.depends_on.len()),
        Style::default().fg(Color::DarkGray),
    )));
    if node.depends_on.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for dep in &node.depends_on {
            let dep_node = app.graph.find_node(dep);
            let color = dep_node
                .map(|n| App::role_color(&n.role))
                .unwrap_or(Color::White);
            lines.push(Line::from(vec![
                Span::styled("  \u{2192} ", Style::default().fg(Color::Green)),
                Span::styled(dep, Style::default().fg(color)),
            ]));
        }
    }

    lines.push(Line::raw(""));

    // Dependents
    lines.push(Line::from(Span::styled(
        format!("depended by ({}):", node.dependents.len()),
        Style::default().fg(Color::DarkGray),
    )));
    if node.dependents.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for dep in &node.dependents {
            let dep_node = app.graph.find_node(dep);
            let color = dep_node
                .map(|n| App::role_color(&n.role))
                .unwrap_or(Color::White);
            let is_impacted = app.mode == Mode::Impact && app.impact_nodes.contains(dep.as_str());
            let style = if is_impacted {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            lines.push(Line::from(vec![
                Span::styled("  \u{2190} ", Style::default().fg(Color::Yellow)),
                Span::styled(dep, style),
            ]));
        }
    }

    // Transitive stats
    let trans_deps = app.graph.transitive_deps(&node.name);
    let trans_rdeps = app.graph.transitive_rdeps(&node.name);
    if trans_deps.len() != node.depends_on.len() || trans_rdeps.len() != node.dependents.len() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "{} direct / {} transitive deps",
                node.depends_on.len(),
                trans_deps.len()
            ),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "{} direct / {} transitive dependents",
                node.dependents.len(),
                trans_rdeps.len()
            ),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines)
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_edit_panel(f: &mut Frame, edit: &super::app::EditState, area: Rect) {
    let block = Block::default()
        .title(" Edit Manifest ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        &edit.node_name,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    // Field 0: description
    let desc_style = if edit.field_index == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled("description:", desc_style)));
    let desc_val = if edit.editing_text && edit.field_index == 0 {
        format!("  {}_", edit.description)
    } else {
        format!("  {}", edit.description)
    };
    lines.push(Line::from(Span::raw(desc_val)));
    lines.push(Line::raw(""));

    // Field 1: role
    let role_style = if edit.field_index == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("role: ", role_style),
        Span::styled(
            format!("{}", edit.role),
            Style::default().fg(App::role_color(&edit.role)),
        ),
        if edit.field_index == 1 {
            Span::styled(" (Enter to cycle)", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("")
        },
    ]));
    lines.push(Line::raw(""));

    // Field 2: depends_on
    let deps_style = if edit.field_index == 2 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled("depends_on:", deps_style)));
    if edit.depends_on.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", Style::default().fg(Color::DarkGray))));
    } else {
        for dep in &edit.depends_on {
            lines.push(Line::from(Span::raw(format!("  - {}", dep))));
        }
    }
    if edit.field_index == 2 {
        lines.push(Line::from(Span::styled(
            "  (use j/k + Enter in list)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::raw(""));

    // Field 3: provides
    let prov_style = if edit.field_index == 3 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled("provides:", prov_style)));
    let prov_val = if edit.editing_text && edit.field_index == 3 {
        format!("  {}_", edit.provides.join(", "))
    } else {
        format!("  {}", edit.provides.join(", "))
    };
    lines.push(Line::from(Span::raw(prov_val)));

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("[s]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" save  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
    ]));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Tree => "TREE",
        Mode::PathSelect => "PATH",
        Mode::Impact => "IMPACT",
    };

    let mode_color = match app.mode {
        Mode::Normal => Color::Cyan,
        Mode::Tree => Color::Green,
        Mode::PathSelect => Color::Yellow,
        Mode::Impact => Color::Red,
    };

    let keybinds = match app.mode {
        Mode::Normal => "t:tree  p:path  i:impact  s:scan  a:assemble  v:verify  e:edit  c:check  /:search  r:role  ?:help",
        Mode::Tree => "Enter:expand  j/k:nav  Esc:back",
        Mode::PathSelect => "Enter:select target  j/k:nav  Esc:back",
        Mode::Impact => "j/k:nav  Esc:back",
    };

    let extra = if app.edit_state.is_some() {
        " [EDITING]"
    } else if app.search.active {
        " [SEARCH]"
    } else {
        ""
    };

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .bg(mode_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(extra, Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("  {}  ", keybinds),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{} nodes", app.graph.nodes.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let para = Paragraph::new(status);
    f.render_widget(para, area);
}

fn draw_popup(f: &mut Frame, popup: &super::app::ActionOutput, area: Rect) {
    // Center popup
    let popup_width = area.width.saturating_sub(10).min(80);
    let popup_height = area.height.saturating_sub(6).min(30);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", popup.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let lines: Vec<Line> = popup
        .lines
        .iter()
        .map(|l| {
            let style = if l.contains("[PASS]") || l.contains("passed") {
                Style::default().fg(Color::Green)
            } else if l.contains("[FAIL]") || l.contains("failed") || l.contains("violation") {
                Style::default().fg(Color::Red)
            } else if l.contains("[ERROR]") {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(l.as_str(), style))
        })
        .collect();

    let mut footer_lines = lines;
    footer_lines.push(Line::raw(""));
    footer_lines.push(Line::from(Span::styled(
        "Press Esc or q to close",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(footer_lines)
        .scroll((popup.scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let popup_width = area.width.saturating_sub(10).min(60);
    let popup_height = area.height.saturating_sub(6).min(25);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  j/k or Up/Down  Navigate list"),
        Line::from("  Tab             Switch focus (list/detail)"),
        Line::from("  /               Search/filter nodes"),
        Line::from("  r               Cycle role filter"),
        Line::raw(""),
        Line::from(Span::styled("View Modes", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  t               Tree view (expand deps)"),
        Line::from("  p               Path finder (select two nodes)"),
        Line::from("  i               Impact analysis (reverse deps)"),
        Line::from("  Esc             Return to normal mode"),
        Line::raw(""),
        Line::from(Span::styled("Actions", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  s               Scan selected repo"),
        Line::from("  a               Assemble graph"),
        Line::from("  v               Verify selected repo"),
        Line::from("  e               Edit manifest"),
        Line::from("  c               Check graph consistency"),
        Line::raw(""),
        Line::from(Span::styled("General", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  ?               Toggle this help"),
        Line::from("  q               Quit"),
    ];

    let para = Paragraph::new(help_text).block(block);
    f.render_widget(para, popup_area);
}
