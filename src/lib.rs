use std::cell::OnceCell;
use std::iter::repeat_n;

#[cfg(feature = "sqlx")]
use sqlx::Executor;
#[cfg(feature = "sqlx")]
use sqlx::sqlite::SqliteQueryResult;

pub use modelite_macros::{BaseModel, Model};

pub struct QuotedColumns<'a> {
    columns: &'a [&'a str],
    data: OnceCell<String>
}

impl<'a> QuotedColumns<'a> {
    const fn new(columns: &'a [&'a str]) -> Self {
        Self {
            columns,
            data: OnceCell::new()
        }
    }

    fn get(&self) -> &str {
        &self.data.get_or_init(|| {
            self.columns.iter().map(|c| String::from("\"") + c + "\"").collect::<Vec<_>>().join(",")
        })
    }
}

impl<'a> std::fmt::Display for QuotedColumns<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get())
    }
}

pub struct ColumnsTemplate {
    len: usize,
    data: OnceCell<String>,
}

impl ColumnsTemplate {
    const fn new(len: usize) -> Self {
        Self {
            len,
            data: OnceCell::new()
        }
    }

    fn get(&self) -> &str {
        &self.data.get_or_init(|| {
            repeat_n("?", self.len).collect::<Vec<_>>().join(",")
        })
    }
}

impl std::fmt::Display for ColumnsTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get())
    }
}

pub trait BaseModel {
    const COLUMNS: &'static [&'static str];
    const QUOTED_COLUMNS: QuotedColumns<'static> = QuotedColumns::new(&Self::COLUMNS);
    const COLUMNS_TEMPLATE: ColumnsTemplate = ColumnsTemplate::new(Self::COLUMNS.len());

    fn create_table_sql() -> String;

    fn table_name() -> String {
        std::any::type_name::<Self>()
            .split("::")
            .last()
            .unwrap()
            .to_string()
    }

    fn insert_sql_template() -> String {
        format!("insert into {} ({}) values ({})", Self::table_name(), Self::QUOTED_COLUMNS, Self::COLUMNS_TEMPLATE)
    }

    fn select_sql() -> String {
        format!("select {} from \"{}\"", Self::QUOTED_COLUMNS, Self::table_name())
    }

    fn drop_table_sql() -> String {
        format!("drop table if exists \"{}\"", Self::table_name())
    }
}

#[cfg(feature = "sqlx")]
#[allow(async_fn_in_trait)]
pub trait Model: BaseModel {
    async fn insert_bulk<'e, 's, E>(e: E, values: impl IntoIterator<Item = &'s Self>) -> Result<SqliteQueryResult, sqlx::Error>
        where
            Self: 'e,
            E: Executor<'e, Database = sqlx::Sqlite>,
            'e: 's
        ;

    async fn create_table<'e, E>(e: E) -> Result<SqliteQueryResult, sqlx::Error>
        where
            Self: 'e,
            E: Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(&Self::create_table_sql()).execute(e).await
    }

    async fn drop_table<'e, E>(e: E) -> Result<SqliteQueryResult, sqlx::Error>
        where
            Self: 'e,
            E: Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(&Self::drop_table_sql()).execute(e).await
    }
}
