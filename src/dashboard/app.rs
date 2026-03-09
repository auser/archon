use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::graph::{Graph, GraphNode, GraphViolation};
use crate::manifest::{Manifest, Role};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Tree,
    PathSelect,
    Impact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

#[derive(Debug)]
pub struct StatefulList {
    pub items: Vec<String>,
    pub all_items: Vec<String>,
    pub state: ListState,
    pub role_filter: Option<Role>,
}

impl StatefulList {
    fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            all_items: items.clone(),
            items,
            state,
            role_filter: None,
        }
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.state
            .selected()
            .and_then(|i| self.items.get(i))
            .map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TreeRow {
    pub name: String,
    pub depth: usize,
    pub is_expanded: bool,
    pub has_children: bool,
}

#[derive(Debug)]
pub struct TreeState {
    pub root: Option<String>,
    pub expanded: HashSet<String>,
    pub flat_rows: Vec<TreeRow>,
    pub cursor: usize,
}

impl TreeState {
    fn new() -> Self {
        Self {
            root: None,
            expanded: HashSet::new(),
            flat_rows: Vec::new(),
            cursor: 0,
        }
    }

    pub fn rebuild(&mut self, graph: &Graph) {
        self.flat_rows.clear();
        if let Some(root) = self.root.clone() {
            self.build_tree(graph, &root, 0);
        }
    }

    fn build_tree(&mut self, graph: &Graph, name: &str, depth: usize) {
        let node = graph.find_node(name);
        let children: Vec<String> = node.map(|n| n.depends_on.clone()).unwrap_or_default();
        let has_children = !children.is_empty();
        let is_expanded = self.expanded.contains(name);

        self.flat_rows.push(TreeRow {
            name: name.to_string(),
            depth,
            is_expanded,
            has_children,
        });

        if is_expanded {
            for child in children {
                self.build_tree(graph, &child, depth + 1);
            }
        }
    }

    pub fn toggle_expand(&mut self, graph: &Graph) {
        if let Some(row) = self.flat_rows.get(self.cursor) {
            if row.has_children {
                let name = row.name.clone();
                if self.expanded.contains(&name) {
                    self.expanded.remove(&name);
                } else {
                    self.expanded.insert(name);
                }
                self.rebuild(graph);
            }
        }
    }

    pub fn cursor_down(&mut self) {
        if !self.flat_rows.is_empty() && self.cursor < self.flat_rows.len() - 1 {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
}

#[derive(Debug)]
pub struct PathFinderState {
    pub from: Option<String>,
    pub to: Option<String>,
    pub result: Option<Vec<String>>,
}

impl PathFinderState {
    fn new() -> Self {
        Self {
            from: None,
            to: None,
            result: None,
        }
    }

    pub fn reset(&mut self) {
        self.from = None;
        self.to = None;
        self.result = None;
    }
}

#[derive(Debug)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
}

impl SearchState {
    fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct ActionOutput {
    pub title: String,
    pub lines: Vec<String>,
    pub scroll: u16,
}

impl ActionOutput {
    pub fn new(title: &str, output: &str) -> Self {
        Self {
            title: title.to_string(),
            lines: output.lines().map(|l| l.to_string()).collect(),
            scroll: 0,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

#[derive(Debug)]
pub struct EditState {
    pub node_name: String,
    pub description: String,
    pub role: Role,
    pub depends_on: Vec<String>,
    pub provides: Vec<String>,
    pub field_index: usize, // which field is selected (0=desc, 1=role, 2=deps, 3=provides)
    pub editing_text: bool, // currently typing in a text field
}

impl EditState {
    pub fn from_node(node: &GraphNode) -> Self {
        Self {
            node_name: node.name.clone(),
            description: node.description.clone(),
            role: node.role.clone(),
            depends_on: node.depends_on.clone(),
            provides: node.provides.clone(),
            field_index: 0,
            editing_text: false,
        }
    }

    pub fn next_field(&mut self) {
        self.field_index = (self.field_index + 1) % 4;
    }

    pub fn prev_field(&mut self) {
        self.field_index = if self.field_index == 0 {
            3
        } else {
            self.field_index - 1
        };
    }

    pub fn cycle_role(&mut self) {
        self.role = match self.role {
            Role::Core => Role::Extension,
            Role::Extension => Role::Tool,
            Role::Tool => Role::Service,
            Role::Service => Role::Library,
            Role::Library => Role::Core,
        };
    }
}

pub struct App {
    pub graph: Graph,
    pub manifests: Vec<(PathBuf, Manifest)>,
    pub root: PathBuf,
    pub registry: PathBuf,

    // Navigation
    pub mode: Mode,
    pub focus: Focus,
    pub node_list: StatefulList,
    pub detail_scroll: u16,

    // View state
    pub tree_state: TreeState,
    pub path_finder: PathFinderState,
    pub impact_nodes: HashSet<String>,
    pub search: SearchState,

    // Management state
    pub action_output: Option<ActionOutput>,
    pub edit_state: Option<EditState>,
    pub violations: Vec<GraphViolation>,

    // UI state
    pub show_help: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        graph: Graph,
        manifests: Vec<(PathBuf, Manifest)>,
        root: PathBuf,
        registry: PathBuf,
    ) -> Self {
        let names: Vec<String> = graph.nodes.iter().map(|n| n.name.clone()).collect();
        Self {
            graph,
            manifests,
            root,
            registry,
            mode: Mode::Normal,
            focus: Focus::List,
            node_list: StatefulList::new(names),
            detail_scroll: 0,
            tree_state: TreeState::new(),
            path_finder: PathFinderState::new(),
            impact_nodes: HashSet::new(),
            search: SearchState::new(),
            action_output: None,
            edit_state: None,
            violations: Vec::new(),
            show_help: false,
            should_quit: false,
        }
    }

    pub fn selected_node(&self) -> Option<&GraphNode> {
        self.node_list
            .selected_name()
            .and_then(|name| self.graph.find_node(name))
    }

    pub fn apply_filter(&mut self) {
        let query = self.search.query.to_lowercase();
        self.node_list.items = self
            .node_list
            .all_items
            .iter()
            .filter(|name| {
                // Search filter
                if !query.is_empty() && !name.to_lowercase().contains(&query) {
                    return false;
                }
                // Role filter
                if let Some(ref role) = self.node_list.role_filter {
                    if let Some(node) = self.graph.find_node(name) {
                        return node.role == *role;
                    }
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        // Reset selection
        if self.node_list.items.is_empty() {
            self.node_list.state.select(None);
        } else {
            self.node_list.state.select(Some(0));
        }
    }

    pub fn cycle_role_filter(&mut self) {
        self.node_list.role_filter = match self.node_list.role_filter {
            None => Some(Role::Core),
            Some(Role::Core) => Some(Role::Extension),
            Some(Role::Extension) => Some(Role::Tool),
            Some(Role::Tool) => Some(Role::Service),
            Some(Role::Service) => Some(Role::Library),
            Some(Role::Library) => None,
        };
        self.apply_filter();
    }

    pub fn enter_tree_mode(&mut self) {
        if let Some(name) = self.node_list.selected_name().map(|s| s.to_string()) {
            self.mode = Mode::Tree;
            self.tree_state.root = Some(name.clone());
            self.tree_state.expanded.clear();
            self.tree_state.expanded.insert(name);
            self.tree_state.cursor = 0;
            self.tree_state.rebuild(&self.graph);
        }
    }

    pub fn enter_path_mode(&mut self) {
        if let Some(name) = self.node_list.selected_name().map(|s| s.to_string()) {
            self.mode = Mode::PathSelect;
            self.path_finder.reset();
            self.path_finder.from = Some(name);
        }
    }

    pub fn select_path_target(&mut self) {
        if let Some(name) = self.node_list.selected_name().map(|s| s.to_string()) {
            self.path_finder.to = Some(name);
            if let (Some(from), Some(to)) = (&self.path_finder.from, &self.path_finder.to) {
                self.path_finder.result = self.graph.find_path(from, to);
            }
        }
    }

    pub fn enter_impact_mode(&mut self) {
        if let Some(name) = self.node_list.selected_name() {
            self.mode = Mode::Impact;
            let rdeps = self.graph.transitive_rdeps(name);
            self.impact_nodes = rdeps.into_iter().collect();
        }
    }

    pub fn run_check(&mut self) {
        self.violations = self.graph.check();
        let output = if self.violations.is_empty() {
            format!("Graph is consistent ({} nodes)", self.graph.nodes.len())
        } else {
            let mut lines = Vec::new();
            for v in &self.violations {
                lines.push(format!("violation: {}", v));
            }
            lines.push(String::new());
            lines.push(format!(
                "{} violation(s) in {} nodes",
                self.violations.len(),
                self.graph.nodes.len()
            ));
            lines.join("\n")
        };
        self.action_output = Some(ActionOutput::new("Check Results", &output));
    }

    pub fn run_verify(&mut self) {
        let name = match self.node_list.selected_name() {
            Some(n) => n.to_string(),
            None => return,
        };

        // Find manifest path for this node
        let manifest_entry = self.manifests.iter().find(|(_, m)| m.name == name);
        let (repo_path, manifest) = match manifest_entry {
            Some((p, m)) => (p.clone(), m.clone()),
            None => {
                self.action_output = Some(ActionOutput::new(
                    "Verify",
                    &format!("No manifest found for '{}'", name),
                ));
                return;
            }
        };

        if manifest.rules.is_empty() {
            self.action_output = Some(ActionOutput::new(
                &format!("Verify: {}", name),
                "No rules defined.",
            ));
            return;
        }

        let mut output_lines = Vec::new();
        output_lines.push(format!(
            "Verifying {} ({} rules)...",
            name,
            manifest.rules.len()
        ));
        output_lines.push(String::new());

        let mut failures = 0u32;
        for rule in &manifest.rules {
            let desc = rule.description.as_deref().unwrap_or(&rule.id);
            let result = std::process::Command::new("sh")
                .arg("-c")
                .arg(&rule.run)
                .current_dir(&repo_path)
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    output_lines.push(format!("  [PASS] {}", desc));
                }
                Ok(out) => {
                    failures += 1;
                    output_lines.push(format!("  [FAIL] {}", desc));
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    for line in stderr.lines().take(10) {
                        output_lines.push(format!("    {}", line));
                    }
                }
                Err(e) => {
                    failures += 1;
                    output_lines.push(format!("  [ERROR] {}: {}", desc, e));
                }
            }
        }

        output_lines.push(String::new());
        if failures == 0 {
            output_lines.push(format!("All {} rules passed.", manifest.rules.len()));
        } else {
            output_lines.push(format!(
                "{}/{} rules failed.",
                failures,
                manifest.rules.len()
            ));
        }

        self.action_output = Some(ActionOutput::new(
            &format!("Verify: {}", name),
            &output_lines.join("\n"),
        ));
    }

    pub fn run_scan(&mut self) {
        let name = match self.node_list.selected_name() {
            Some(n) => n.to_string(),
            None => return,
        };

        let manifest_entry = self.manifests.iter().find(|(_, m)| m.name == name);
        let repo_path = match manifest_entry {
            Some((p, _)) => p.clone(),
            None => {
                self.action_output = Some(ActionOutput::new(
                    "Scan",
                    &format!("No manifest found for '{}'", name),
                ));
                return;
            }
        };

        let result = std::process::Command::new("archon")
            .args(["scan", "--path"])
            .arg(&repo_path)
            .output();

        let output = match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                format!("{}{}", stdout, stderr)
            }
            Err(e) => format!("Failed to run archon scan: {}", e),
        };

        self.action_output = Some(ActionOutput::new(&format!("Scan: {}", name), &output));
    }

    pub fn run_assemble(&mut self) {
        let result = std::process::Command::new("archon")
            .args(["assemble", "--root"])
            .arg(&self.root)
            .arg("--registry")
            .arg(&self.registry)
            .output();

        let output = match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                format!("{}{}", stdout, stderr)
            }
            Err(e) => format!("Failed to run archon assemble: {}", e),
        };

        self.action_output = Some(ActionOutput::new("Assemble", &output));

        // Reload graph
        self.reload_graph();
    }

    pub fn save_edit(&mut self) {
        let edit = match self.edit_state.take() {
            Some(e) => e,
            None => return,
        };

        // Find and update the manifest
        if let Some((repo_path, manifest)) = self
            .manifests
            .iter_mut()
            .find(|(_, m)| m.name == edit.node_name)
        {
            manifest.description = edit.description;
            manifest.role = edit.role;
            manifest.depends_on = edit.depends_on;
            manifest.provides = edit.provides;

            if let Err(e) = manifest.save(repo_path) {
                self.action_output = Some(ActionOutput::new(
                    "Edit Error",
                    &format!("Failed to save manifest: {}", e),
                ));
                return;
            }
        }

        // Re-assemble graph from updated manifests
        let manifests: Vec<Manifest> = self.manifests.iter().map(|(_, m)| m.clone()).collect();
        self.graph = Graph::assemble(manifests);

        // Refresh node list
        let names: Vec<String> = self.graph.nodes.iter().map(|n| n.name.clone()).collect();
        self.node_list.all_items = names;
        self.apply_filter();
    }

    fn reload_graph(&mut self) {
        let graph_path = self.registry.join("graph.yaml");
        if let Ok(graph) = Graph::load(&graph_path) {
            self.graph = graph;
            let names: Vec<String> = self.graph.nodes.iter().map(|n| n.name.clone()).collect();
            self.node_list.all_items = names;
            self.apply_filter();
        }
    }

    pub fn role_color(role: &Role) -> ratatui::style::Color {
        use ratatui::style::Color;
        match role {
            Role::Core => Color::Cyan,
            Role::Extension => Color::Green,
            Role::Tool => Color::Yellow,
            Role::Service => Color::Magenta,
            Role::Library => Color::Blue,
        }
    }
}
