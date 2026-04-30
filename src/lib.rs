use std::cell::OnceCell;
use std::iter::repeat_n;

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
        format!("{} values ({})", Self::insert_sql(), Self::COLUMNS_TEMPLATE)
    }

    fn insert_sql() -> String {
        format!("insert into \"{}\" ({})", Self::table_name(), Self::QUOTED_COLUMNS)
    }

    fn select_sql() -> String {
        format!("select {} from \"{}\"", Self::QUOTED_COLUMNS, Self::table_name())
    }

    fn drop_table_sql() -> String {
        format!("drop table if exists \"{}\"", Self::table_name())
    }
}

/// Execute all queries in a bulk query (see `Model::insert_bulk`).
///
/// # Example
/// ```ignore
/// # use modelite::{Model, execute_all};
/// # let conn = sqlx::SqlitePool::connect(":memory:").await.unwrap();
///
/// #[derive(Model)]
/// struct Person {
///     name: String,
///     age: u32
/// }
///
/// let query = Person::insert_bulk(&conn, vec![Person { name: "John Johnson".to_string(), age: 43 }, /* more entries*/].iter()).await.unwrap();
/// execute_all!(&conn, query).await.unwrap();
/// ```
#[cfg(feature = "sqlx")]
#[macro_export]
macro_rules! execute_all {
    ($bulk_query_iterator: expr, $executor: expr) => {
        async {
            for query in $bulk_query_iterator {
                query.execute($executor).await?;
            }

            Ok::<(), sqlx::Error>(())
        }
        // ::futures::future::try_join_all($bulk_query_iterator.map(async |q: ::modelite::ModeliteQuery<'_>| q.execute($executor).await))
    }
}

#[cfg(feature = "sqlx")]
mod sqlx_feature {
    use std::borrow::Cow;
    use std::ptr::NonNull;

    use futures_core::stream::BoxStream;
    use libsqlite3_sys::sqlite3;
    use sqlx::query::{Query, QueryAs};
    use sqlx::{Executor, FromRow, Pool, QueryBuilder, Sqlite, SqliteConnection};
    use sqlx::pool::PoolConnection;
    use sqlx::sqlite::{LockedSqliteHandle, SqliteQueryResult, SqliteRow};

    use crate::BaseModel;

    pub trait IntoRawSqliteHandle {
        fn into_raw_sqlite_handle(self) -> impl std::future::Future<Output = Result<NonNull<sqlite3>, sqlx::Error>> + Send;
    }

    impl IntoRawSqliteHandle for &'_ mut LockedSqliteHandle<'_> {
        async fn into_raw_sqlite_handle(self) -> Result<NonNull<sqlite3>, sqlx::Error> {
            Ok(self.as_raw_handle())
        }
    }

    impl IntoRawSqliteHandle for &'_ mut PoolConnection<Sqlite> {
        async fn into_raw_sqlite_handle(self) -> Result<NonNull<sqlite3>, sqlx::Error> {
            self.lock_handle().await
                .map(|mut r| r.as_raw_handle())
        }
    }

    impl IntoRawSqliteHandle for &'_ Pool<Sqlite> {
        async fn into_raw_sqlite_handle(self) -> Result<NonNull<sqlite3>, sqlx::Error> {
            let mut conn = match self.acquire().await {
                Ok(conn) => conn,
                Err(e) => return Err(e.into())
            };
            let mut lock_handle = match conn.lock_handle().await {
                Ok(conn) => conn,
                Err(e) => return Err(e)
            };

            Ok(lock_handle.as_raw_handle())
        }
    }

    impl IntoRawSqliteHandle for &'_ mut SqliteConnection {
        async fn into_raw_sqlite_handle(self) -> Result<NonNull<sqlite3>, sqlx::Error> {
            self.lock_handle().await
                .map(|mut r| r.as_raw_handle())
        }
    }

    type SqliteQuery<'q> = Query<'q, Sqlite, <Sqlite as sqlx::Database>::Arguments<'q>>;
    type SqliteQueryAs<'q, T> = QueryAs<'q, Sqlite, T, <Sqlite as sqlx::Database>::Arguments<'q>>;

    pub enum ModeliteQuery<'a> {
        Builder(QueryBuilder<'a, Sqlite>),
        Query(Cow<'a, str>),
    }

    impl<'a> ModeliteQuery<'a> {
        /// Execute the query
        pub async fn execute<'c, E: Executor<'c, Database = Sqlite>>(mut self, e: E) -> Result<SqliteQueryResult, sqlx::Error> {
            self.build().execute(e).await
        }

        /// Fetch all rows into a vec
        pub async fn fetch_all<'c, E: Executor<'c, Database = Sqlite>>(mut self, e: E) -> Result<Vec<SqliteRow>, sqlx::Error> {
            self.build().fetch_all(e).await
        }

        /// Fetch one row
        pub async fn fetch_one<'c, E: Executor<'c, Database = Sqlite>>(mut self, e: E) -> Result<SqliteRow, sqlx::Error> {
            self.build().fetch_one(e).await
        }

        /// Fetch all rows as a stream
        pub async fn fetch<E: Executor<'a, Database = Sqlite>>(&'a mut self, e: E) -> BoxStream<'a, Result<SqliteRow, sqlx::Error>> {
            self.build().fetch(e)
        }

        pub async fn fetch_all_as<'c, E: Executor<'c, Database = Sqlite>, T>(&'a mut self, e: E) -> Result<Vec<T>, sqlx::Error>
            where T: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row> + Send + Sync + Unpin,
        {
            self.build_as().fetch_all(e).await
        }

        pub async fn fetch_one_as<'c, E: Executor<'c, Database = Sqlite>, T>(&'a mut self, e: E) -> Result<T, sqlx::Error>
            where T: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row> + Send + Sync + Unpin,
        {
            self.build_as().fetch_one(e).await
        }

        pub async fn fetch_as<'c, E: Executor<'a, Database = Sqlite> + 'c, T>(&'a mut self, e: E) -> BoxStream<'c, Result<T, sqlx::Error>>
            where
                T: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row> + Send + Sync + Unpin + 'c,
                'a: 'c
        {
            self.build_as().fetch(e)
        }

        /// Builds and returns the query as an `sqlx::Query`
        pub fn build(&mut self) -> SqliteQuery<'_> {
            match self {
                ModeliteQuery::Builder(qb) => qb.build(),
                ModeliteQuery::Query(query_str) => sqlx::query(query_str),
            }
        }

        pub fn build_as<'c, T>(&'a mut self) -> SqliteQueryAs<'a, T>
            where T: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row>,
        {
            match self {
                ModeliteQuery::Builder(qb) => qb.build_query_as(),
                ModeliteQuery::Query(query_str) => sqlx::query_as(query_str),
            }
        }
    }

    pub struct BulkQueryIterator<'s, M: Model + ?Sized + 's, I: Iterator<Item = &'s M>> {
        iter: I,
        chunk_size: usize,
    }

    impl<'s, M: Model + 's, I: Iterator<Item = &'s M>> Iterator for BulkQueryIterator<'s, M, I> {
        type Item = ModeliteQuery<'s>;

        fn next(&mut self) -> Option<Self::Item> {
            let mut chunk = self.iter.by_ref().take(self.chunk_size).peekable();
            if chunk.peek().is_none() { return None; }
            Some(M::insert(chunk))
        }
    }

    impl <'s, M: Model + 's, I: Iterator<Item = &'s M>> BulkQueryIterator<'s, M, I> {
        pub async fn execute_all(self, e: &sqlx::Pool<Sqlite>) -> Result<Vec<SqliteQueryResult>, sqlx::Error> {
            let mut vec = Vec::new();
            for r in self.map(|q| q.execute(e)) {
                vec.push(r.await?)
            }
            return Ok(vec);
        }
    }

    pub trait Model: BaseModel + 'static {
        fn insert<'s>(values: impl IntoIterator<Item = &'s Self>) -> ModeliteQuery<'s>;

        // Chunks values up into multiple inserts if necessary (based on SQLITE_LIMIT_VARIABLE_NUMBER)
        fn insert_bulk<'s, H: IntoRawSqliteHandle, I: IntoIterator<Item = &'s Self>>(h: H, values: I) -> impl Future<Output = Result<BulkQueryIterator<'s, Self, I::IntoIter>, sqlx::Error>> {
            impl_insert_bulk(h, values)
        }

        fn create_table<'s>() -> ModeliteQuery<'s> {
            ModeliteQuery::Query(Self::create_table_sql().into())
        }

        fn drop_table<'s>() -> ModeliteQuery<'s> {
            ModeliteQuery::Query(Self::drop_table_sql().into())
        }

        fn select_all<'s>() -> ModeliteQuery<'s> {
            ModeliteQuery::Query(Self::select_sql().into())
        }
    }

    #[inline]
    async fn impl_insert_bulk<'s, H: IntoRawSqliteHandle, I: IntoIterator<Item = &'s M>, M: Model + ?Sized>(h: H, values: I) -> Result<BulkQueryIterator<'s, M, I::IntoIter>, sqlx::Error> {
        use libsqlite3_sys::{sqlite3_limit, SQLITE_LIMIT_VARIABLE_NUMBER};

        let raw_conn: NonNull<sqlite3> = h.into_raw_sqlite_handle().await?;

        let max_variables = unsafe { sqlite3_limit(raw_conn.as_ptr(), SQLITE_LIMIT_VARIABLE_NUMBER, -1) } as usize;
        let variables_per_row = M::COLUMNS.len();

        let max_variables = max_variables - (max_variables % variables_per_row);

        Ok(BulkQueryIterator {
            iter: values.into_iter(),
            chunk_size: max_variables / variables_per_row,
        })
    }
}

pub use sqlx_feature::*;
