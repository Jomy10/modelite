use std::ptr::NonNull;

use libsqlite3_sys::{sqlite3, sqlite3_limit, SQLITE_LIMIT_VARIABLE_NUMBER};
use sqlx::Sqlite;
use sqlx::sqlite::SqliteRow;

use crate::{BaseModel, BulkQueryIterator, IntoRawSqliteHandle, SqliteQuery, SqliteQueryAs};

pub trait Model: BaseModel + 'static {
    fn insert<'s>(value: &'s Self) -> SqliteQuery<'s>;
    fn insert_all_unchecked<'s>(values: impl IntoIterator<Item = &'s Self>) -> sqlx::QueryBuilder<'s, Sqlite>;

    // Chunks values up into multiple inserts if necessary (based on SQLITE_LIMIT_VARIABLE_NUMBER)
    fn insert_bulk<'s, H: IntoRawSqliteHandle, I: IntoIterator<Item = &'s Self>>(h: H, values: I) -> impl Future<Output = Result<BulkQueryIterator<'s, Self, I::IntoIter>, sqlx::Error>> {
        impl_insert_bulk(h, values)
    }

    fn create_table<'s>() -> SqliteQuery<'s> {
        sqlx::query(Self::create_table_sql())
    }

    fn drop_table<'s>() -> SqliteQuery<'s> {
        sqlx::query(Self::drop_table_sql())
    }

    fn select_all<'s>() -> SqliteQueryAs<'s, Self>
        where
            Self: Sized,
            for<'r> Self: sqlx::FromRow<'r, SqliteRow>
    {
        sqlx::query_as(Self::select_sql())
    }
}

#[inline]
async fn impl_insert_bulk<'s, H: IntoRawSqliteHandle, I: IntoIterator<Item = &'s M>, M: Model + ?Sized>(h: H, values: I) -> Result<BulkQueryIterator<'s, M, I::IntoIter>, sqlx::Error> {

    let raw_conn: NonNull<sqlite3> = h.into_raw_sqlite_handle().await?;

    let max_variables = unsafe { sqlite3_limit(raw_conn.as_ptr(), SQLITE_LIMIT_VARIABLE_NUMBER, -1) } as usize;
    let variables_per_row = M::COLUMNS.len();

    let max_variables = max_variables - (max_variables % variables_per_row);

    Ok(BulkQueryIterator {
        iter: values.into_iter(),
        chunk_size: max_variables / variables_per_row,
    })
}
