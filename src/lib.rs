use serde::{ser::Serializer, Serialize};
use serde_json::{json, Value as JsonValue};
use sqlite::{Connection, Type, Value as SqliteValue};
use std::{collections::HashMap, sync::Mutex};
use tauri::{
  command,
  plugin::{Builder, TauriPlugin},
  Manager, Runtime, State,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Sqlite(#[from] sqlite::Error),
  #[error("database {0} not opened")]
  DatabaseNotOpened(String),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_string().as_ref())
  }
}

type Result<T> = std::result::Result<T, Error>;

macro_rules! bind {
  ($statement:expr, $index:expr, $value:expr) => {
    if $value.is_null() {
      $statement.bind($index + 1, $value.as_null()).unwrap();
    } else if $value.is_i64() {
      $statement.bind($index + 1, $value.as_i64()).unwrap();
    } else if $value.is_boolean() {
      $statement
        .bind($index + 1, $value.as_bool().unwrap() as i64)
        .unwrap();
    } else if $value.is_f64() {
      $statement.bind($index + 1, $value.as_f64()).unwrap();
    } else if $value.is_string() {
      $statement
        .bind($index + 1, $value.as_str().to_owned())
        .unwrap();
    }
  };
}

#[derive(Default)]
struct SqliteMap(Mutex<HashMap<String, Connection>>);

#[command]
fn open(state: State<'_, SqliteMap>, path: String) -> Result<bool> {
  let connection = Connection::open(&path)?;
  state.0.lock().unwrap().insert(path.clone(), connection);
  Ok(true)
}

#[command]
fn execute(state: State<'_, SqliteMap>, path: String, sql: String) -> Result<bool> {
  let mut map = state.0.lock().unwrap();
  let connection = map.get_mut(&path).ok_or(Error::DatabaseNotOpened(path))?;
  connection.execute(&sql)?;
  Ok(true)
}

#[command]
fn execute2(
  state: State<'_, SqliteMap>,
  path: String,
  sql: String,
  values: Vec<JsonValue>,
) -> Result<bool> {
  let mut map = state.0.lock().unwrap();
  let connection = map.get_mut(&path).ok_or(Error::DatabaseNotOpened(path))?;
  let mut statement = connection.prepare(&sql)?;
  if values.get(0).unwrap().is_array() {
    for value in values {
      for (i, v) in value.as_array().unwrap().iter().enumerate() {
        bind!(statement, i, v);
      }
      statement.next()?;
      statement.reset()?;
    }
    Ok(true)
  } else {
    for (i, v) in values.iter().enumerate() {
      bind!(statement, i, v);
    }
    statement.next()?;
    Ok(true)
  }
}

#[command]
fn select(
  state: State<'_, SqliteMap>,
  path: String,
  sql: String,
  values: Vec<JsonValue>,
) -> Result<Vec<HashMap<String, JsonValue>>> {
  let mut map = state.0.lock().unwrap();
  let connection = map.get_mut(&path).ok_or(Error::DatabaseNotOpened(path))?;
  let statement = connection.prepare(&sql)?;

  let mut names: Vec<String> = Vec::new();
  for name in statement.column_names() {
    names.push(name.to_string());
  }

  let mut params = Vec::new();
  for value in values {
    if value.is_null() {
      params.push(SqliteValue::Null);
    } else if value.is_i64() {
      params.push(SqliteValue::Integer(value.as_i64().unwrap()));
    } else if value.is_boolean() {
      params.push(SqliteValue::Integer(value.as_bool().unwrap() as i64));
    } else if value.is_f64() {
      params.push(SqliteValue::Float(value.as_f64().unwrap()));
    } else if value.is_string() {
      params.push(SqliteValue::String(value.as_str().unwrap().to_owned()));
    }
  }

  let mut cursor = statement.into_cursor();
  cursor.bind(&params)?;

  let mut rows = Vec::new();
  while let Some(_row) = cursor.next()? {
    let mut row = HashMap::default();
    for (i, name) in names.iter().enumerate() {
      let value = _row.get(i).unwrap();
      let v = match value.kind() {
        Type::Float => json!(value.as_float().unwrap()),
        Type::Integer => json!(value.as_integer().unwrap()),
        Type::String => json!(value.as_string().unwrap().to_owned()),
        Type::Binary => json!(value.as_binary().unwrap()),
        _ => JsonValue::Null,
      };
      row.insert(name.to_owned(), v);
    }
    rows.push(row);
  }
  Ok(rows)
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("sqlite")
    .invoke_handler(tauri::generate_handler![open, execute, execute2, select])
    .setup(|app| {
      app.manage(SqliteMap::default());
      Ok(())
    })
    .build()
}
