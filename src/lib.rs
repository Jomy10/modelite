#[cfg(feature = "sqlx")]
use sqlx::Executor;
#[cfg(feature = "sqlx")]
use sqlx::sqlite::SqliteQueryResult;

pub use modelite_macros::{BaseModel, Model};

pub trait BaseModel<const N: usize> {
    const COLUMNS: [&'static str; N];

    fn create_table_sql() -> String;

    fn table_name() -> String {
        std::any::type_name::<Self>()
            .split("::")
            .last()
            .unwrap()
            .to_string()
    }

    fn insert_sql() -> String {
        format!("insert into {} ({}) values ({})", Self::table_name(), Self::COLUMNS.join(","), ["?"; N].join(","))
    }

    fn select_sql() -> String {
        format!("select {} from {}", Self::COLUMNS.join(","), Self::table_name())
    }
}

#[cfg(feature = "sqlx")]
pub trait Model<const N: usize>: BaseModel<N> {
    #[allow(async_fn_in_trait)]
    async fn insert_bulk<'e, 'a, E>(e: E, values: impl IntoIterator<Item = &'a Self>) -> Result<SqliteQueryResult, sqlx::Error>
        where
            Self: 'a,
            E: Executor<'a, Database = sqlx::Sqlite>,
        ;
}
