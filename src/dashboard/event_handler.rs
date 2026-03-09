use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, EditState, Focus, Mode};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // If action output popup is showing, handle it first
    if app.action_output.is_some() {
        handle_popup(app, key);
        return;
    }

    // Help overlay
    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.show_help = false,
            _ => {}
        }
        return;
    }

    // Edit mode
    if app.edit_state.is_some() {
        handle_edit(app, key);
        return;
    }

    // Search mode
    if app.search.active {
        handle_search(app, key);
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::Tree => handle_tree(app, key),
        Mode::PathSelect => handle_path_select(app, key),
        Mode::Impact => handle_impact(app, key),
    }
}

fn handle_popup(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.action_output = None;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut popup) = app.action_output {
                popup.scroll_down();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut popup) = app.action_output {
                popup.scroll_up();
            }
        }
        _ => {}
    }
}

fn handle_edit(app: &mut App, key: KeyEvent) {
    let edit = match app.edit_state.as_mut() {
        Some(e) => e,
        None => return,
    };

    if edit.editing_text {
        // Currently editing a text field
        match key.code {
            KeyCode::Esc => {
                edit.editing_text = false;
            }
            KeyCode::Enter => {
                edit.editing_text = false;
            }
            KeyCode::Backspace => match edit.field_index {
                0 => {
                    edit.description.pop();
                }
                3 => {
                    // Edit provides as comma-separated
                    let mut text = edit.provides.join(", ");
                    text.pop();
                    edit.provides = text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            },
            KeyCode::Char(c) => match edit.field_index {
                0 => edit.description.push(c),
                3 => {
                    let mut text = edit.provides.join(", ");
                    text.push(c);
                    edit.provides = text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            },
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.edit_state = None;
        }
        KeyCode::Enter => {
            match edit.field_index {
                0 | 3 => {
                    // Start editing text field
                    edit.editing_text = true;
                }
                1 => {
                    // Cycle role
                    edit.cycle_role();
                }
                2 => {
                    // Toggle dep: add/remove currently selected node from deps
                    if let Some(selected) = app.node_list.selected_name() {
                        let selected = selected.to_string();
                        if selected != edit.node_name {
                            if let Some(pos) = edit.depends_on.iter().position(|d| *d == selected) {
                                edit.depends_on.remove(pos);
                            } else {
                                edit.depends_on.push(selected);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if edit.field_index == 2 {
                // Navigate node list to select deps
                app.node_list.next();
            } else {
                edit.next_field();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if edit.field_index == 2 {
                app.node_list.previous();
            } else {
                edit.prev_field();
            }
        }
        KeyCode::Tab => {
            edit.next_field();
        }
        KeyCode::Char('s') => {
            // Save
            app.save_edit();
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search.active = false;
        }
        KeyCode::Enter => {
            app.search.active = false;
        }
        KeyCode::Backspace => {
            app.search.query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search.query.push(c);
            app.apply_filter();
        }
        _ => {}
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('/') => {
            app.search.active = true;
            app.search.query.clear();
        }

        // Navigation
        KeyCode::Down | KeyCode::Char('j') => match app.focus {
            Focus::List => app.node_list.next(),
            Focus::Detail => app.detail_scroll = app.detail_scroll.saturating_add(1),
        },
        KeyCode::Up | KeyCode::Char('k') => match app.focus {
            Focus::List => app.node_list.previous(),
            Focus::Detail => app.detail_scroll = app.detail_scroll.saturating_sub(1),
        },
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::List => Focus::Detail,
                Focus::Detail => Focus::List,
            };
        }

        // Filters
        KeyCode::Char('r') => app.cycle_role_filter(),

        // View modes
        KeyCode::Char('t') => app.enter_tree_mode(),
        KeyCode::Char('p') => app.enter_path_mode(),
        KeyCode::Char('i') => app.enter_impact_mode(),

        // Management actions
        KeyCode::Char('c') => app.run_check(),
        KeyCode::Char('v') => app.run_verify(),
        KeyCode::Char('s') => app.run_scan(),
        KeyCode::Char('a') => app.run_assemble(),
        KeyCode::Char('e') => {
            if let Some(node) = app.selected_node() {
                app.edit_state = Some(EditState::from_node(node));
            }
        }

        _ => {}
    }
}

fn handle_tree(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
        }
        KeyCode::Down | KeyCode::Char('j') => app.tree_state.cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => app.tree_state.cursor_up(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.tree_state.toggle_expand(&app.graph);
        }
        KeyCode::Char('?') => app.show_help = true,
        _ => {}
    }
}

fn handle_path_select(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
            app.path_finder.reset();
        }
        KeyCode::Down | KeyCode::Char('j') => app.node_list.next(),
        KeyCode::Up | KeyCode::Char('k') => app.node_list.previous(),
        KeyCode::Enter => {
            app.select_path_target();
        }
        KeyCode::Char('?') => app.show_help = true,
        _ => {}
    }
}

fn handle_impact(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
            app.impact_nodes.clear();
        }
        KeyCode::Down | KeyCode::Char('j') => app.node_list.next(),
        KeyCode::Up | KeyCode::Char('k') => app.node_list.previous(),
        KeyCode::Char('?') => app.show_help = true,
        _ => {}
    }
}
