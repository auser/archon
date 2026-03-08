use serde::{Deserialize, Serialize};

/// Date-based standards version in "YYYY.MM" format (e.g., "2026.03").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StandardsVersion(pub String);

impl StandardsVersion {
    /// Parse and validate a standards version string.
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(format!("expected YYYY.MM format, got: {s}"));
        }

        let year: u32 = parts[0]
            .parse()
            .map_err(|_| format!("invalid year: {}", parts[0]))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| format!("invalid month: {}", parts[1]))?;

        if !(2020..=2100).contains(&year) {
            return Err(format!("year out of range: {year}"));
        }
        if !(1..=12).contains(&month) {
            return Err(format!("month out of range: {month}"));
        }

        Ok(Self(s.to_string()))
    }

    /// Check if this version matches the latest.
    pub fn is_current(&self, latest: &Self) -> bool {
        self == latest
    }

    /// Human-readable drift description if behind.
    pub fn drift(&self, latest: &Self) -> Option<String> {
        if self >= latest {
            None
        } else {
            Some(format!(
                "repo is at {}, latest is {}",
                self.0, latest.0
            ))
        }
    }

    /// Get the current standards version based on today's date.
    pub fn current() -> Self {
        let now = chrono::Utc::now();
        Self(format!("{}.{:02}", now.format("%Y"), now.format("%m")))
    }
}

impl std::fmt::Display for StandardsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let v = StandardsVersion::parse("2026.03").unwrap();
        assert_eq!(v.0, "2026.03");
    }

    #[test]
    fn parse_invalid_format() {
        assert!(StandardsVersion::parse("2026").is_err());
        assert!(StandardsVersion::parse("2026.13").is_err());
        assert!(StandardsVersion::parse("2026.00").is_err());
        assert!(StandardsVersion::parse("abc.03").is_err());
    }

    #[test]
    fn ordering() {
        let v1 = StandardsVersion::parse("2025.12").unwrap();
        let v2 = StandardsVersion::parse("2026.03").unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn drift_detection() {
        let old = StandardsVersion::parse("2025.12").unwrap();
        let new = StandardsVersion::parse("2026.03").unwrap();
        assert!(old.drift(&new).is_some());
        assert!(new.drift(&new).is_none());
    }
}
