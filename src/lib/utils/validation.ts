/**
 * Input validation utilities
 * Provides validation functions for user inputs
 */

/**
 * Validate search query
 */
export function validateSearchQuery(query: string): { valid: boolean; error?: string } {
  if (!query || typeof query !== "string") {
    return { valid: false, error: "Search query is required" };
  }

  const trimmed = query.trim();
  if (trimmed.length === 0) {
    return { valid: false, error: "Search query cannot be empty" };
  }

  if (trimmed.length > 200) {
    return { valid: false, error: "Search query is too long (max 200 characters)" };
  }

  // Check for potentially malicious patterns
  if (/[<>{}[\]\\]/.test(trimmed)) {
    return { valid: false, error: "Search query contains invalid characters" };
  }

  return { valid: true };
}

/**
 * Sanitize string input
 */
export function sanitizeString(input: string): string {
  return input
    .trim()
    .replace(/[<>{}[\]\\]/g, "") // Remove potentially dangerous characters
    .slice(0, 1000); // Limit length
}

/**
 * Validate numeric ID
 */
export function validateId(id: unknown): { valid: boolean; value?: number; error?: string } {
  if (typeof id !== "number" && typeof id !== "string") {
    return { valid: false, error: "ID must be a number" };
  }

  const numId = typeof id === "string" ? parseInt(id, 10) : id;

  if (isNaN(numId) || numId <= 0 || !Number.isInteger(numId)) {
    return { valid: false, error: "ID must be a positive integer" };
  }

  return { valid: true, value: numId };
}

/**
 * Validate date string
 */
export function validateDate(dateString: string): { valid: boolean; error?: string } {
  if (!dateString || typeof dateString !== "string") {
    return { valid: false, error: "Date is required" };
  }

  const date = new Date(dateString);
  if (isNaN(date.getTime())) {
    return { valid: false, error: "Invalid date format" };
  }

  return { valid: true };
}

/**
 * Validate URL
 */
export function validateUrl(url: string): { valid: boolean; error?: string } {
  if (!url || typeof url !== "string") {
    return { valid: false, error: "URL is required" };
  }

  try {
    new URL(url);
    return { valid: true };
  } catch {
    return { valid: false, error: "Invalid URL format" };
  }
}
