/**
 * Shared database connection utility
 * Provides singleton database instance with migration handling
 */

import Database from "@tauri-apps/plugin-sql";
import { logger } from "./logger";

// Use separate database in dev mode to avoid breaking production data
const DB_NAME = import.meta.env.DEV ? "sqlite:tvc_dev.db" : "sqlite:tvc.db";

let dbInstance: Database | null = null;
let dbInitialized = false;

/**
 * Get or create the shared database instance.
 * 
 * This function implements a singleton pattern to ensure only one database
 * connection is used across the application. It handles:
 * - Database connection initialization
 * - Migration execution (currently migration 2 for scheduled_date column)
 * - Error handling and logging
 * 
 * @returns A promise that resolves to the Database instance
 * @throws Error if database cannot be loaded or initialized
 */
export async function getDatabase(): Promise<Database> {
  if (!dbInstance) {
    try {
      dbInstance = await Database.load(DB_NAME);
      logger.debug("Database connection established", { dbName: DB_NAME });
    } catch (error) {
      logger.error("Failed to load database", error);
      throw error;
    }
  }

  if (!dbInitialized) {
    await initializeDatabase(dbInstance);
    dbInitialized = true;
  }

  return dbInstance;
}

/**
 * Initialize database with migrations
 */
async function initializeDatabase(db: Database): Promise<void> {
  try {
    // Ensure migrations table exists
    await db.execute(`
      CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY);
    `);

    // Check if migration 2 (scheduled_date column) has been applied
    const migrations = await db.select<{ version: number }[]>(
      "SELECT version FROM _migrations WHERE version = 2"
    );

    if (migrations.length === 0) {
      try {
        await db.execute("ALTER TABLE episodes ADD COLUMN scheduled_date TEXT");
        await db.execute("INSERT OR IGNORE INTO _migrations (version) VALUES (2)");
        logger.info("Applied migration: scheduled_date column");
      } catch (error) {
        // Column might already exist, which is fine
        logger.debug("Migration 2 may already be applied", error);
      }
    }
  } catch (error) {
    logger.error("Failed to initialize database", error);
    throw error;
  }
}

/**
 * Close database connection (useful for cleanup)
 */
export async function closeDatabase(): Promise<void> {
  if (dbInstance) {
    // Tauri SQL plugin doesn't have explicit close, but we can clear the reference
    dbInstance = null;
    dbInitialized = false;
    logger.debug("Database connection closed");
  }
}
