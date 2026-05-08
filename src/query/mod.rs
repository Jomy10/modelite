use sqlx::Sqlite;
use sqlx::query::{Query, QueryAs};

moduse!(bulk);

pub(crate) type SqliteQuery<'q> = Query<'q, Sqlite, <Sqlite as sqlx::Database>::Arguments<'q>>;
pub(crate) type SqliteQueryAs<'q, T> = QueryAs<'q, Sqlite, T, <Sqlite as sqlx::Database>::Arguments<'q>>;
