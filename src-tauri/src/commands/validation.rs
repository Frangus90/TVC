//! Input validation utilities for Rust commands
//! Mirrors the validation.ts patterns from the frontend

/// Validate that an ID is a positive integer
pub fn validate_id(id: i64) -> Result<(), String> {
    if id <= 0 {
        return Err("ID must be a positive integer".to_string());
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
    if query.contains('<') || query.contains('>') || 
       query.contains('{') || query.contains('}') ||
       query.contains('[') || query.contains(']') ||
       query.contains('\\') {
        return Err("Search query contains invalid characters".to_string());
    }
    
    Ok(())
}

/// Validate date string format (YYYY-MM-DD)
pub fn validate_date(date: &str) -> Result<(), String> {
    if date.is_empty() {
        return Err("Date is required".to_string());
    }
    
    // Basic format check
    if !date.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err("Invalid date format".to_string());
    }
    
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("Date must be in YYYY-MM-DD format".to_string());
    }
    
    // Validate year, month, day ranges
    if let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u32>(),
        parts[2].parse::<u32>(),
    ) {
        if year < 1900 || year > 2100 {
            return Err("Year must be between 1900 and 2100".to_string());
        }
        if month < 1 || month > 12 {
            return Err("Month must be between 1 and 12".to_string());
        }
        if day < 1 || day > 31 {
            return Err("Day must be between 1 and 31".to_string());
        }
    } else {
        return Err("Date must contain valid numbers".to_string());
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
