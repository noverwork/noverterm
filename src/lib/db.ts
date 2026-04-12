import Database from '@tauri-apps/plugin-sql'

const DB_PATH = 'sqlite:novercpa.db'

let _db: Database | null = null

export async function getDb(): Promise<Database> {
  if (!_db) {
    _db = await Database.load(DB_PATH)
  }
  return _db
}

export async function execute(query: string, bindValues?: unknown[]) {
  const db = await getDb()
  return db.execute(query, bindValues)
}

export async function select<T>(query: string, bindValues?: unknown[]): Promise<T> {
  const db = await getDb()
  return db.select<T>(query, bindValues)
}
