/**
 * Centralized logging utility.
 * 
 * Provides structured logging with levels and environment-based filtering.
 * In production, only warnings and errors are logged. In development, all
 * log levels are enabled.
 * 
 * Features:
 * - Log level filtering (debug, info, warn, error)
 * - Environment-based filtering (dev vs production)
 * - Log history (last 100 entries)
 * - Structured logging with optional data objects
 * 
 * Usage:
 * ```ts
 * import { logger } from './utils/logger';
 * logger.info('User logged in', { userId: 123 });
 * logger.error('Failed to save', error);
 * ```
 */

type LogLevel = "debug" | "info" | "warn" | "error";

interface LogEntry {
  level: LogLevel;
  message: string;
  data?: unknown;
  timestamp: string;
}

class Logger {
  private isDev: boolean;
  private logHistory: LogEntry[] = [];
  private maxHistorySize = 100;

  constructor() {
    this.isDev = import.meta.env.DEV;
  }

  private shouldLog(level: LogLevel): boolean {
    // In production, only log warnings and errors
    if (!this.isDev) {
      return level === "warn" || level === "error";
    }
    return true;
  }

  private formatMessage(level: LogLevel, message: string, data?: unknown): string {
    const prefix = `[${level.toUpperCase()}]`;
    if (data !== undefined) {
      return `${prefix} ${message} ${JSON.stringify(data, null, 2)}`;
    }
    return `${prefix} ${message}`;
  }

  private log(level: LogLevel, message: string, data?: unknown): void {
    if (!this.shouldLog(level)) {
      return;
    }

    const entry: LogEntry = {
      level,
      message,
      data,
      timestamp: new Date().toISOString(),
    };

    // Keep limited history
    this.logHistory.push(entry);
    if (this.logHistory.length > this.maxHistorySize) {
      this.logHistory.shift();
    }

    // Output to console with appropriate method
    const formattedMessage = this.formatMessage(level, message, data);
    switch (level) {
      case "debug":
      case "info":
        console.log(formattedMessage);
        break;
      case "warn":
        console.warn(formattedMessage);
        break;
      case "error":
        console.error(formattedMessage);
        break;
    }
  }

  debug(message: string, data?: unknown): void {
    this.log("debug", message, data);
  }

  info(message: string, data?: unknown): void {
    this.log("info", message, data);
  }

  warn(message: string, data?: unknown): void {
    this.log("warn", message, data);
  }

  error(message: string, data?: unknown): void {
    this.log("error", message, data);
  }

  /**
   * Get recent log history (useful for debugging)
   */
  getHistory(level?: LogLevel): LogEntry[] {
    if (level) {
      return this.logHistory.filter((entry) => entry.level === level);
    }
    return [...this.logHistory];
  }

  /**
   * Clear log history
   */
  clearHistory(): void {
    this.logHistory = [];
  }
}

// Export singleton instance
export const logger = new Logger();
