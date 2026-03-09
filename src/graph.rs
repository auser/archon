use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::broadcast::Broadcast;
use crate::manifest::{Manifest, Role};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub name: String,
    pub description: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub generated_at: String,
    pub nodes: Vec<GraphNode>,
}

#[derive(Debug)]
pub struct GraphViolation {
    pub node: String,
    pub message: String,
}

impl std::fmt::Display for GraphViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.node, self.message)
    }
}

impl Graph {
    /// Assemble a graph from a collection of manifests.
    pub fn assemble(manifests: Vec<Manifest>) -> Self {
        let mut nodes: Vec<GraphNode> = manifests
            .iter()
            .map(|m| GraphNode {
                name: m.name.clone(),
                description: m.description.clone(),
                role: m.role.clone(),
                depends_on: m.depends_on.clone(),
                provides: m.provides.clone(),
                dependents: vec![],
            })
            .collect();

        // Compute inverse edges (dependents).
        let dep_map: HashMap<&str, Vec<&str>> = manifests
            .iter()
            .map(|m| (m.name.as_str(), m.depends_on.iter().map(|s| s.as_str()).collect()))
            .collect();

        for node in &mut nodes {
            for (source, deps) in &dep_map {
                if deps.contains(&node.name.as_str()) {
                    node.dependents.push(source.to_string());
                }
            }
            node.dependents.sort();
        }

        nodes.sort_by(|a, b| a.name.cmp(&b.name));

        let generated_at = chrono::Utc::now().to_rfc3339();
        Graph {
            generated_at,
            nodes,
        }
    }

    /// Load a previously assembled graph from YAML.
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let graph: Graph =
            serde_yaml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
        Ok(graph)
    }

    /// Save the graph as YAML.
    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("serializing graph")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        std::fs::write(path, yaml).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Validate graph consistency. Returns a list of violations.
    pub fn check(&self) -> Vec<GraphViolation> {
        let mut violations = Vec::new();
        let names: HashSet<&str> = self.nodes.iter().map(|n| n.name.as_str()).collect();

        // Check all depends_on targets exist.
        for node in &self.nodes {
            for dep in &node.depends_on {
                if !names.contains(dep.as_str()) {
                    violations.push(GraphViolation {
                        node: node.name.clone(),
                        message: format!("depends on '{}' which is not in the graph", dep),
                    });
                }
            }
        }

        // Detect cycles.
        if let Some(cycle) = self.detect_cycle() {
            violations.push(GraphViolation {
                node: cycle[0].clone(),
                message: format!("dependency cycle: {}", cycle.join(" -> ")),
            });
        }

        violations
    }

    /// Find a node by name.
    pub fn find_node(&self, name: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.name == name)
    }

    /// Collect all transitive dependencies of a node (BFS).
    pub fn transitive_deps(&self, name: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(name.to_string());
        visited.insert(name.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(node) = self.find_node(&current) {
                for dep in &node.depends_on {
                    if visited.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        visited.remove(name);
        let mut result: Vec<String> = visited.into_iter().collect();
        result.sort();
        result
    }

    /// Collect all transitive reverse dependencies (who depends on this, recursively).
    pub fn transitive_rdeps(&self, name: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(name.to_string());
        visited.insert(name.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(node) = self.find_node(&current) {
                for dep in &node.dependents {
                    if visited.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        visited.remove(name);
        let mut result: Vec<String> = visited.into_iter().collect();
        result.sort();
        result
    }

    /// Find shortest dependency path from `from` to `to` (BFS on depends_on edges).
    /// Returns None if no path exists.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }

        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        queue.push_back(from.to_string());
        visited.insert(from.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(node) = self.find_node(&current) {
                for dep in &node.depends_on {
                    if visited.insert(dep.clone()) {
                        parent.insert(dep.clone(), current.clone());
                        if dep == to {
                            // Reconstruct path.
                            let mut path = vec![to.to_string()];
                            let mut cur = to.to_string();
                            while let Some(p) = parent.get(&cur) {
                                path.push(p.clone());
                                cur = p.clone();
                            }
                            path.reverse();
                            return Some(path);
                        }
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        None
    }

    fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node.name.as_str()) {
                if let Some(cycle) =
                    self.dfs_cycle(&node.name, &mut visited, &mut in_stack, &mut path)
                {
                    return Some(cycle);
                }
            }
        }
        None
    }

    fn dfs_cycle<'a>(
        &'a self,
        name: &'a str,
        visited: &mut HashSet<&'a str>,
        in_stack: &mut HashSet<&'a str>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        visited.insert(name);
        in_stack.insert(name);
        path.push(name);

        if let Some(node) = self.find_node(name) {
            for dep in &node.depends_on {
                if !visited.contains(dep.as_str()) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, in_stack, path) {
                        return Some(cycle);
                    }
                } else if in_stack.contains(dep.as_str()) {
                    // Found cycle — extract the cycle path.
                    let cycle_start = path.iter().position(|&n| n == dep.as_str()).unwrap();
                    let mut cycle: Vec<String> =
                        path[cycle_start..].iter().map(|s| s.to_string()).collect();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        in_stack.remove(name);
        None
    }
}

/// Collect manifests from sibling directories.
pub fn collect_manifests(root: &Path) -> Result<Vec<(std::path::PathBuf, Manifest)>> {
    let mut results = Vec::new();

    for entry in std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("archon.yaml");
            if manifest_path.exists() {
                match Manifest::load(&path) {
                    Ok(manifest) => results.push((path, manifest)),
                    Err(e) => {
                        eprintln!(
                            "warning: skipping {}: {}",
                            manifest_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    Ok(results)
}

/// Collect broadcasts from a registry broadcasts/ directory.
pub fn collect_broadcasts(broadcasts_dir: &Path) -> Result<HashMap<String, Broadcast>> {
    let mut broadcasts = HashMap::new();

    if !broadcasts_dir.exists() {
        return Ok(broadcasts);
    }

    for entry in
        std::fs::read_dir(broadcasts_dir).with_context(|| format!("reading {}", broadcasts_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let broadcast: Broadcast = serde_yaml::from_str(&content)
                .with_context(|| format!("parsing {}", path.display()))?;
            broadcasts.insert(broadcast.repo.clone(), broadcast);
        }
    }

    Ok(broadcasts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifests() -> Vec<Manifest> {
        vec![
            Manifest {
                name: "core".into(),
                description: "Core library".into(),
                role: Role::Core,
                depends_on: vec![],
                provides: vec!["core-api".into()],
                crates: None,
                auto_update: None,
                registry: None,
                rules: vec![],
            },
            Manifest {
                name: "extension".into(),
                description: "Extension".into(),
                role: Role::Extension,
                depends_on: vec!["core".into()],
                provides: vec!["ext-api".into()],
                crates: None,
                auto_update: None,
                registry: None,
                rules: vec![],
            },
            Manifest {
                name: "tool".into(),
                description: "Tool".into(),
                role: Role::Tool,
                depends_on: vec!["core".into(), "extension".into()],
                provides: vec![],
                crates: None,
                auto_update: None,
                registry: None,
                rules: vec![],
            },
        ]
    }

    #[test]
    fn assemble_graph() {
        let graph = Graph::assemble(test_manifests());
        assert_eq!(graph.nodes.len(), 3);

        let core = graph.find_node("core").unwrap();
        assert_eq!(core.dependents, vec!["extension", "tool"]);

        let ext = graph.find_node("extension").unwrap();
        assert_eq!(ext.dependents, vec!["tool"]);
        assert_eq!(ext.depends_on, vec!["core"]);

        let tool = graph.find_node("tool").unwrap();
        assert!(tool.dependents.is_empty());
    }

    #[test]
    fn check_valid_graph() {
        let graph = Graph::assemble(test_manifests());
        let violations = graph.check();
        assert!(violations.is_empty());
    }

    #[test]
    fn check_missing_dependency() {
        let manifests = vec![Manifest {
            name: "orphan".into(),
            description: "Orphan".into(),
            role: Role::Extension,
            depends_on: vec!["nonexistent".into()],
            provides: vec![],
            crates: None,
            auto_update: None,
            registry: None,
            rules: vec![],
        }];
        let graph = Graph::assemble(manifests);
        let violations = graph.check();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("nonexistent"));
    }

    #[test]
    fn detect_cycle() {
        let manifests = vec![
            Manifest {
                name: "a".into(),
                description: "A".into(),
                role: Role::Core,
                depends_on: vec!["b".into()],
                provides: vec![],
                crates: None,
                auto_update: None,
                registry: None,
                rules: vec![],
            },
            Manifest {
                name: "b".into(),
                description: "B".into(),
                role: Role::Core,
                depends_on: vec!["a".into()],
                provides: vec![],
                crates: None,
                auto_update: None,
                registry: None,
                rules: vec![],
            },
        ];
        let graph = Graph::assemble(manifests);
        let violations = graph.check();
        assert!(violations.iter().any(|v| v.message.contains("cycle")));
    }

    #[test]
    fn graph_yaml_round_trip() {
        let graph = Graph::assemble(test_manifests());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.yaml");

        graph.save(&path).unwrap();
        let loaded = Graph::load(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 3);
        assert_eq!(loaded.nodes[0].name, "core");
    }
}
