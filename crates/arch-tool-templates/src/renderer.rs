use std::collections::HashMap;

/// Simple template renderer that replaces {{key}} placeholders with values.
pub fn render(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

// Embedded template files
pub const REPO_META_TEMPLATE: &str = include_str!("../templates/hologram.repo.yaml.tpl");
pub const AGENTS_MD_TEMPLATE: &str = include_str!("../templates/init/AGENTS.md.tpl");
pub const CLAUDE_MD_TEMPLATE: &str = include_str!("../templates/init/CLAUDE.md.tpl");
pub const ARCHITECTURE_MD_TEMPLATE: &str = include_str!("../templates/init/architecture.md.tpl");
pub const DEVELOPMENT_MD_TEMPLATE: &str = include_str!("../templates/init/development.md.tpl");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_replaces_vars() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "my-project".to_string());
        vars.insert("version".to_string(), "2026.03".to_string());

        let result = render("project: {{name}}, version: {{version}}", &vars);
        assert_eq!(result, "project: my-project, version: 2026.03");
    }

    #[test]
    fn render_leaves_unknown_vars() {
        let vars = HashMap::new();
        let result = render("{{unknown}} stays", &vars);
        assert_eq!(result, "{{unknown}} stays");
    }
}
