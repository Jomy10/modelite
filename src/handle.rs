use std::ptr::NonNull;

use libsqlite3_sys::sqlite3;
use sqlx::{Pool, Sqlite, SqliteConnection};
use sqlx::pool::PoolConnection;
use sqlx::sqlite::LockedSqliteHandle;

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
