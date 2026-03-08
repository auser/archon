use colored::Colorize;
use arch_tool_core::standards_version::StandardsVersion;
use arch_tool_policy::evaluator::CheckResult;
use arch_tool_policy::model::Severity;
use serde::Serialize;

/// Full conformance report for a repository.
#[derive(Debug)]
pub struct ConformanceReport {
    pub results: Vec<CheckResult>,
    pub repo_name: String,
    pub standards_version: Option<StandardsVersion>,
}

impl ConformanceReport {
    pub fn errors(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.passed && r.severity == Severity::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.passed && r.severity == Severity::Warning)
            .count()
    }

    /// True if no errors (warnings are OK).
    pub fn passed(&self) -> bool {
        self.errors() == 0
    }

    /// True if no errors and no warnings.
    pub fn passed_strict(&self) -> bool {
        self.errors() == 0 && self.warnings() == 0
    }

    pub fn exit_code(&self, strict: bool) -> i32 {
        let failed = if strict {
            !self.passed_strict()
        } else {
            !self.passed()
        };
        if failed { 1 } else { 0 }
    }

    /// Print colored report to stdout.
    pub fn print(&self) {
        println!(
            "\n{} {}",
            "arch-tool verify:".bold(),
            self.repo_name.cyan()
        );

        if let Some(v) = &self.standards_version {
            println!("  standards version: {}", v.0.yellow());
        }
        println!();

        for result in &self.results {
            let icon = if result.passed {
                "✓".green().to_string()
            } else {
                match result.severity {
                    Severity::Error => "✗".red().to_string(),
                    Severity::Warning => "⚠".yellow().to_string(),
                    Severity::Info => "ℹ".blue().to_string(),
                }
            };

            let excepted_tag = if result.excepted {
                " [excepted]".dimmed().to_string()
            } else {
                String::new()
            };

            println!(
                "  {} [{}] {}: {}{}",
                icon,
                result.rule_id.dimmed(),
                result.category.dimmed(),
                result.message,
                excepted_tag
            );
        }

        println!();
        let errors = self.errors();
        let warnings = self.warnings();
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();

        if errors > 0 {
            println!(
                "  {} {}/{} checks passed, {} errors, {} warnings",
                "FAIL".red().bold(),
                passed,
                total,
                errors,
                warnings
            );
        } else if warnings > 0 {
            println!(
                "  {} {}/{} checks passed, {} warnings",
                "PASS".green().bold(),
                passed,
                total,
                warnings
            );
        } else {
            println!(
                "  {} {}/{} checks passed",
                "PASS".green().bold(),
                passed,
                total
            );
        }
        println!();
    }

    /// Serialize report as JSON for CI consumption.
    pub fn to_json(&self) -> String {
        let json_report = JsonReport {
            repo_name: &self.repo_name,
            standards_version: self.standards_version.as_ref().map(|v| v.0.as_str()),
            passed: self.passed(),
            errors: self.errors(),
            warnings: self.warnings(),
            results: self
                .results
                .iter()
                .map(|r| JsonCheckResult {
                    rule_id: &r.rule_id,
                    category: &r.category,
                    severity: format!("{}", r.severity),
                    passed: r.passed,
                    message: &r.message,
                    excepted: r.excepted,
                })
                .collect(),
        };
        serde_json::to_string_pretty(&json_report).unwrap_or_default()
    }
}

#[derive(Serialize)]
struct JsonReport<'a> {
    repo_name: &'a str,
    standards_version: Option<&'a str>,
    passed: bool,
    errors: usize,
    warnings: usize,
    results: Vec<JsonCheckResult<'a>>,
}

#[derive(Serialize)]
struct JsonCheckResult<'a> {
    rule_id: &'a str,
    category: &'a str,
    severity: String,
    passed: bool,
    message: &'a str,
    excepted: bool,
}
