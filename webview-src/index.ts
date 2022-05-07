import { invoke } from '@tauri-apps/api/tauri'

export default class SQLite {
  path: string

  constructor(path: string) {
    this.path = path
  }

  static async open(path: string): Promise<SQLite> {
    return await invoke<string>('plugin:sqlite|open', { path }).then(() => new SQLite(path))
  }

  async execute(sql: string, values?: unknown[]): Promise<boolean> {
    return values ? invoke('plugin:sqlite|execute2', { path: this.path, sql, values }) : invoke('plugin:sqlite|execute', { path: this.path, sql })
  }

  async select<T>(sql: string, values?: unknown[]): Promise<T> {
    return await invoke('plugin:sqlite|select', { path: this.path, sql, values: values ?? [] })
  }
}
