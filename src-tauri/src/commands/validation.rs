//! Input validation utilities for Rust commands
//! Mirrors the validation.ts patterns from the frontend

/// Validate that an ID is a non-zero integer (positive for API entries, negative for manual entries)
pub fn validate_id(id: i64) -> Result<(), String> {
    if id == 0 {
        return Err("ID cannot be zero".to_string());
    }
    Ok(())
}

/// Validate search query string
#[allow(dead_code)]
pub fn validate_search_query(query: &str) -> Result<(), String> {
    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    if query.len() > 200 {
        return Err("Search query is too long (max 200 characters)".to_string());
    }

    // Check for potentially malicious patterns
    if query.contains('<')
        || query.contains('>')
        || query.contains('{')
        || query.contains('}')
        || query.contains('[')
        || query.contains(']')
        || query.contains('\\')
    {
        return Err("Search query contains invalid characters".to_string());
    }

    Ok(())
}

/// Validate date string format (YYYY-MM-DD)
pub fn validate_date(date: &str) -> Result<(), String> {
    use chrono::Datelike;
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "Enter a valid date in YYYY-MM-DD format".to_string())?;
    if parsed.format("%Y-%m-%d").to_string() != date || !(1900..=2100).contains(&parsed.year()) {
        return Err("Enter a date in YYYY-MM-DD format between 1900 and 2100".into());
    }
    Ok(())
}

/// Validate URL (for opening external links)
#[allow(dead_code)]
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is required".to_string());
    }

    // Only allow https:// URLs for security
    if !url.starts_with("https://") {
        return Err("Only HTTPS URLs are allowed".to_string());
    }

    // Basic URL format validation (check for valid structure)
    if url.len() < 8 || !url.contains('.') {
        return Err("Invalid URL format".to_string());
    }

    // Check for dangerous patterns
    if url.contains("javascript:") || url.contains("data:") || url.contains("<script") {
        return Err("URL contains potentially dangerous content".to_string());
    }

    Ok(())
}

/// Validate port number
pub fn validate_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("Port cannot be zero".to_string());
    }
    Ok(())
}
