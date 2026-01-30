# Database Connection Strategy

This document describes the database connection strategy used in TVC.

## Overview

TVC uses **two separate database connection mechanisms**:

1. **sqlx with SqlitePool** - Primary mechanism for most database operations
2. **tauri-plugin-sql** - Used for specific Tauri plugin integrations

## Connection Pooling

### sqlx Pool (`src-tauri/src/db/connection.rs`)

- **Singleton Pattern**: The connection pool is stored in a static `OnceLock<Mutex<Option<SqlitePool>>>` to ensure only one pool is created and reused across all database operations.
- **Lazy Initialization**: The pool is created on first access and reused for subsequent calls.
- **Health Checking**: Before reusing a pool, we check if it's closed and recreate if necessary.
- **Max Connections**: Limited to 5 concurrent connections to prevent resource exhaustion.

### tauri-plugin-sql

- Used for specific Tauri plugin features that require direct SQL access.
- Configured in `tauri.conf.json` with preload: `["sqlite:tvc.db"]`
- This creates a separate connection mechanism that doesn't share the sqlx pool.

## Best Practices

1. **Always use `get_pool()`** from `connection.rs` for sqlx operations - never create new pools directly.
2. **Reuse connections** - The singleton pattern ensures efficient connection reuse.
3. **Handle errors gracefully** - Database errors should be converted to `AppError` for consistent error handling.
4. **Use transactions** - For multi-step operations, use explicit transactions with proper rollback on errors.

## Migration Strategy

Database migrations are handled via SQL files in `src-tauri/migrations/` and applied automatically on startup. The `migration_repair.rs` module ensures backwards compatibility by handling schema checksum mismatches.

## Future Considerations

- Consider consolidating to a single connection strategy if possible
- Monitor connection pool usage in production
- Consider connection pool metrics/monitoring
