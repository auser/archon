use serde::{Deserialize, Serialize};

/// An Architecture Decision Record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    pub number: u32,
    pub title: String,
    pub status: AdrStatus,
    pub date: String,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

impl std::fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "Proposed"),
            Self::Accepted => write!(f, "Accepted"),
            Self::Deprecated => write!(f, "Deprecated"),
            Self::Superseded => write!(f, "Superseded"),
        }
    }
}

/// An exception record — approved deviation from a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionRecord {
    pub id: String,
    pub rule: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    pub created: String,
}

impl ExceptionRecord {
    /// Check if this exception has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = &self.expires {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            exp < &today
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_status_display() {
        assert_eq!(AdrStatus::Proposed.to_string(), "Proposed");
        assert_eq!(AdrStatus::Accepted.to_string(), "Accepted");
    }

    #[test]
    fn exception_not_expired_without_date() {
        let exc = ExceptionRecord {
            id: "EXC-001".into(),
            rule: "STR-001".into(),
            reason: "test".into(),
            expires: None,
            created: "2026-01-01".into(),
        };
        assert!(!exc.is_expired());
    }

    #[test]
    fn exception_expired_in_past() {
        let exc = ExceptionRecord {
            id: "EXC-001".into(),
            rule: "STR-001".into(),
            reason: "test".into(),
            expires: Some("2020-01-01".into()),
            created: "2019-01-01".into(),
        };
        assert!(exc.is_expired());
    }
}
