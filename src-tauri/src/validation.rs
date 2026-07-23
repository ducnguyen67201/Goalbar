use base64::Engine as _;
use sha2::{Digest, Sha256};
use url::Url;

use crate::domain::browser::{BrowserBounds, BrowserRunLimits};
use crate::error::{AppError, AppResult};

pub fn require_non_empty(value: &str, field: &str, max_chars: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} cannot be empty")));
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::Validation(format!(
            "{field} must be at most {max_chars} characters"
        )));
    }
    Ok(trimmed.to_owned())
}

pub fn payload_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

pub fn allowlisted_external_url(value: &str) -> AppResult<Url> {
    let url = Url::parse(value).map_err(|error| AppError::Validation(error.to_string()))?;
    if url.scheme() != "https" {
        return Err(AppError::Validation(
            "external URL must use HTTPS".to_owned(),
        ));
    }
    let allowed = matches!(
        url.host_str(),
        Some(
            "x.com"
                | "www.x.com"
                | "twitter.com"
                | "www.twitter.com"
                | "reddit.com"
                | "www.reddit.com"
                | "linkedin.com"
                | "www.linkedin.com"
        )
    );
    if !allowed {
        return Err(AppError::Validation(
            "external URL host is not allowlisted".to_owned(),
        ));
    }
    Ok(url)
}

pub fn validate_browser_bounds(bounds: BrowserBounds) -> AppResult<()> {
    if !bounds.x.is_finite()
        || !bounds.y.is_finite()
        || !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || bounds.x < 0.0
        || bounds.y < 0.0
        || !(80.0..=10_000.0).contains(&bounds.width)
        || !(80.0..=10_000.0).contains(&bounds.height)
    {
        return Err(AppError::Validation(
            "browser bounds are outside the supported desktop area".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_browser_run_limits(limits: &BrowserRunLimits) -> AppResult<()> {
    if !(1..=500).contains(&limits.maximum_items) {
        return Err(AppError::Validation(
            "maximumItems must be between 1 and 500".to_owned(),
        ));
    }
    if !(1..=100).contains(&limits.maximum_steps) {
        return Err(AppError::Validation(
            "maximumSteps must be between 1 and 100".to_owned(),
        ));
    }
    if let Some(value) = &limits.earliest_date {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
            AppError::Validation("earliestDate must be an RFC 3339 timestamp".to_owned())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        allowlisted_external_url, payload_hash, require_non_empty, validate_browser_bounds,
        validate_browser_run_limits,
    };
    use crate::domain::browser::{BrowserBounds, BrowserRunLimits};

    #[test]
    fn validates_required_text() {
        assert!(require_non_empty("  hello ", "text", 8).is_ok());
        assert!(require_non_empty(" ", "text", 8).is_err());
        assert!(require_non_empty("123456789", "text", 8).is_err());
    }

    #[test]
    fn hashes_deterministically() {
        assert_eq!(payload_hash("same"), payload_hash("same"));
        assert_ne!(payload_hash("same"), payload_hash("different"));
    }

    #[test]
    fn allows_only_official_https_hosts() {
        assert!(allowlisted_external_url("https://www.linkedin.com/in/founder").is_ok());
        assert!(allowlisted_external_url("http://www.linkedin.com/in/founder").is_err());
        assert!(allowlisted_external_url("https://linkedin.com.evil.test").is_err());
    }

    #[test]
    fn validates_browser_limits_and_bounds() {
        assert!(
            validate_browser_bounds(BrowserBounds {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            })
            .is_ok()
        );
        assert!(
            validate_browser_bounds(BrowserBounds {
                x: -1.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            })
            .is_err()
        );
        assert!(
            validate_browser_run_limits(&BrowserRunLimits {
                maximum_items: 50,
                maximum_steps: 25,
                earliest_date: None,
            })
            .is_ok()
        );
        assert!(
            validate_browser_run_limits(&BrowserRunLimits {
                maximum_items: 501,
                maximum_steps: 25,
                earliest_date: None,
            })
            .is_err()
        );
    }
}
