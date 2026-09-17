import Database from "@tauri-apps/plugin-sql";

// Numbered Rust migrations are the sole schema owner. Share in-flight loads too.
const DB_NAME = import.meta.env.DEV ? "sqlite:tvc_dev.db" : "sqlite:tvc.db";
let connection: Promise<Database> | null = null;
export function getDatabase(): Promise<Database> {
  if (!connection) {
    const pending = Database.load(DB_NAME).catch((error) => {
      if (connection === pending) connection = null;
      throw error;
    });
    connection = pending;
  }
  return connection;
}
export async function closeDatabase(): Promise<void> {
  const pending = connection;
  connection = null;
  if (pending) await (await pending).close();
}
