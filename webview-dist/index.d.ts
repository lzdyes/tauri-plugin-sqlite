export default class SQLite {
    path: string;
    constructor(path: string);
    static open(path: string): Promise<SQLite>;
    execute(sql: string, values?: unknown[]): Promise<boolean>;
    select<T>(sql: string, values?: unknown[]): Promise<T>;
}
