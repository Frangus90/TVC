/**
 * Centralized application configuration
 * All configurable values should be defined here
 */

export const config = {
  // API Configuration
  api: {
    // Request timeouts (in milliseconds)
    timeout: 30000,
    // Retry configuration
    maxRetries: 3,
    retryDelay: 1000,
  },

  // UI Configuration
  ui: {
    // Toast notification duration (in milliseconds)
    toastDuration: 3000,
    // Search debounce delay (in milliseconds)
    searchDebounceMs: 300,
    // Animation durations (in milliseconds)
    animationDuration: 200,
  },

  // Database Configuration
  database: {
    // Migration version tracking
    currentMigrationVersion: 2,
  },

  // Calendar Configuration
  calendar: {
    // Default view mode
    defaultView: "month" as const,
    // Days to load around current view
    bufferDays: 7,
  },

  // Development Configuration
  dev: {
    // Enable verbose logging in development
    verboseLogging: import.meta.env.DEV,
    // Enable performance monitoring
    performanceMonitoring: import.meta.env.DEV,
  },
} as const;

export type Config = typeof config;
