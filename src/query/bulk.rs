use sqlx::{QueryBuilder, Sqlite};
use sqlx::sqlite::SqliteQueryResult;

use crate::Model;

pub struct BulkQueryIterator<'s, M: Model + ?Sized + 's, I: Iterator<Item = &'s M>> {
    pub(crate) iter: I,
    pub(crate) chunk_size: usize,
}

impl<'s, M: Model + 's, I: Iterator<Item = &'s M>> Iterator for BulkQueryIterator<'s, M, I> {
    type Item = QueryBuilder<'s, Sqlite>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = self.iter.by_ref().take(self.chunk_size).peekable();
        if chunk.peek().is_none() { return None; }
        Some(M::insert_all_unchecked(chunk))
    }
}

impl<'s, M: Model + 's, I: Iterator<Item = &'s M>> BulkQueryIterator<'s, M, I> {
    pub async fn execute_all(self, e: &sqlx::Pool<Sqlite>) -> Result<Vec<SqliteQueryResult>, sqlx::Error> {
        let mut vec = Vec::new();
        for mut query in self {
            vec.push(query.build().execute(e).await?);
        }
        return Ok(vec);
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
            for mut query in $bulk_query_iterator {
                query.build().execute($executor).await?;
            }

            Ok::<(), sqlx::Error>(())
        }
        // ::futures::future::try_join_all($bulk_query_iterator.map(async |q: ::modelite::ModeliteQuery<'_>| q.execute($executor).await))
    }
}
