use crate::errors::{AppError, AppResult};

/// Validate an IPv4 address string.
pub fn validate_ipv4(addr: &str) -> AppResult<()> {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() != 4 {
        return Err(AppError::Validation(format!("Invalid IPv4 address: {}", addr)));
    }
    for part in parts {
        if part.is_empty() {
            return Err(AppError::Validation(format!("Invalid IPv4 address: {}", addr)));
        }
        match part.parse::<u8>() {
            Ok(_) => {}
            Err(_) => {
                return Err(AppError::Validation(format!("Invalid IPv4 address: {}", addr)));
            }
        }
    }
    Ok(())
}

/// Validate an IPv6 address string using std.
pub fn validate_ipv6(addr: &str) -> AppResult<()> {
    if addr.parse::<std::net::Ipv6Addr>().is_err() {
        return Err(AppError::Validation(format!("Invalid IPv6 address: {}", addr)));
    }
    Ok(())
}

/// Validate either IPv4 or IPv6.
pub fn validate_ip(addr: &str) -> AppResult<()> {
    if addr.parse::<std::net::IpAddr>().is_err() {
        return Err(AppError::Validation(format!("Invalid IP address: {}", addr)));
    }
    Ok(())
}

/// Validate a profile id (alphanumeric, dash, underscore).
pub fn validate_profile_id(id: &str) -> AppResult<()> {
    if id.is_empty() || id.len() > 128 {
        return Err(AppError::Validation("Profile id must be 1-128 chars".to_string()));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::Validation(
            "Profile id may only contain alphanumeric, dash, underscore".to_string(),
        ));
    }
    Ok(())
}

/// Validate a human-readable profile name.
pub fn validate_profile_name(name: &str) -> AppResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(AppError::Validation("Profile name must be 1-64 chars".to_string()));
    }
    if trimmed.chars().any(|c| c == '\n' || c == '\r' || c == '\u{0}') {
        return Err(AppError::Validation("Profile name contains invalid characters".to_string()));
    }
    Ok(())
}

/// Validate a DoH URL (https only).
pub fn validate_doh_url(url: &str) -> AppResult<()> {
    if url.is_empty() {
        return Err(AppError::Validation("DoH URL cannot be empty".to_string()));
    }
    if !url.starts_with("https://") {
        return Err(AppError::Validation("DoH URL must use https://".to_string()));
    }
    if url.len() > 512 {
        return Err(AppError::Validation("DoH URL too long".to_string()));
    }
    if url.contains('\n') || url.contains('\r') || url.contains('\u{0}') {
        return Err(AppError::Validation("DoH URL contains invalid characters".to_string()));
    }
    Ok(())
}

/// Validate a DoT hostname.
pub fn validate_dot_host(host: &str) -> AppResult<()> {
    if host.is_empty() {
        return Err(AppError::Validation("DoT host cannot be empty".to_string()));
    }
    if host.len() > 253 {
        return Err(AppError::Validation("DoT host too long".to_string()));
    }
    if host.contains('\n') || host.contains('\r') || host.contains('\u{0}') {
        return Err(AppError::Validation("DoT host contains invalid characters".to_string()));
    }
    Ok(())
}

/// Validate a hostname used for diagnostics.
pub fn validate_host(host: &str) -> AppResult<()> {
    if host.is_empty() || host.len() > 253 {
        return Err(AppError::Validation("Invalid host".to_string()));
    }
    if host.contains('\n') || host.contains('\r') || host.contains('\u{0}') {
        return Err(AppError::Validation("Invalid host".to_string()));
    }
    Ok(())
}

/// Sanitize a generic string for safe display / logging.
pub fn sanitize_output(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{0}' | '\u{7}' | '\u{8}'))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Truncate output to a maximum length for safety.
pub fn truncate_output(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...[truncated]", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_valid_ipv4() {
        assert!(validate_ipv4("1.1.1.1").is_ok());
        assert!(validate_ipv4("8.8.8.8").is_ok());
        assert!(validate_ipv4("255.255.255.255").is_ok());
        assert!(validate_ipv4("0.0.0.0").is_ok());
    }

    #[test]
    fn rejects_invalid_ipv4() {
        assert!(validate_ipv4("1.1.1").is_err());
        assert!(validate_ipv4("1.1.1.1.1").is_err());
        assert!(validate_ipv4("256.1.1.1").is_err());
        assert!(validate_ipv4("a.b.c.d").is_err());
        assert!(validate_ipv4("").is_err());
        assert!(validate_ipv4("...").is_err());
    }

    #[test]
    fn validates_valid_ipv6() {
        assert!(validate_ipv6("::1").is_ok());
        assert!(validate_ipv6("2606:4700:4700::1111").is_ok());
        assert!(validate_ipv6("2001:4860:4860::8888").is_ok());
    }

    #[test]
    fn rejects_invalid_ipv6() {
        assert!(validate_ipv6("2606:4700:4700::1111::1").is_err());
        assert!(validate_ipv6("not-an-ip").is_err());
        assert!(validate_ipv6("").is_err());
    }

    #[test]
    fn validates_ip_either_family() {
        assert!(validate_ip("1.1.1.1").is_ok());
        assert!(validate_ip("::1").is_ok());
        assert!(validate_ip("not-an-ip").is_err());
    }

    #[test]
    fn validates_profile_id() {
        assert!(validate_profile_id("preset_cloudflare").is_ok());
        assert!(validate_profile_id("custom-1_a").is_ok());
        assert!(validate_profile_id("").is_err());
        assert!(validate_profile_id("has space").is_err());
        assert!(validate_profile_id("has/slash").is_err());
        assert!(validate_profile_id(&"a".repeat(129)).is_err());
    }

    #[test]
    fn validates_profile_name() {
        assert!(validate_profile_name("Cloudflare").is_ok());
        assert!(validate_profile_name("My Custom DNS").is_ok());
        assert!(validate_profile_name("  ").is_err());
        assert!(validate_profile_name("line\nbreak").is_err());
        assert!(validate_profile_name("null\u{0}char").is_err());
    }

    #[test]
    fn validates_doh_url() {
        assert!(validate_doh_url("https://cloudflare-dns.com/dns-query").is_ok());
        assert!(validate_doh_url("http://example.com").is_err());
        assert!(validate_doh_url("").is_err());
        assert!(validate_doh_url("https://bad\nurl").is_err());
    }

    #[test]
    fn validates_dot_host() {
        assert!(validate_dot_host("one.one.one.one").is_ok());
        assert!(validate_dot_host("").is_err());
        assert!(validate_dot_host("bad\nhost").is_err());
    }

    #[test]
    fn validates_host() {
        assert!(validate_host("example.com").is_ok());
        assert!(validate_host("8.8.8.8").is_ok());
        assert!(validate_host("").is_err());
        assert!(validate_host(&"a".repeat(254)).is_err());
        assert!(validate_host("bad\nhost").is_err());
    }

    #[test]
    fn sanitizes_output_removes_control() {
        assert_eq!(sanitize_output("hello"), "hello");
        assert_eq!(sanitize_output("hello\u{0}world"), "helloworld");
        assert_eq!(sanitize_output("  trim me  "), "trim me");
    }

    #[test]
    fn truncate_output_respects_max() {
        assert_eq!(truncate_output("abc", 10), "abc");
        assert_eq!(truncate_output("abcdefghij", 3), "abc...[truncated]");
    }
}
