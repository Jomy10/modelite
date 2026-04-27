# Modelite

Modelite defines traits and macros to generate SQL queries as strings, or to be used with
[sqlx](https://github.com/launchbadge/sqlx) (with feature "sqlx", enabled by default).

```rust
#[derive(Model)]
struct Person {
    name: String,
    age: u32,
    nickname: Option<String>
}

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    
    Person::create_table(&pool);
    Person::insert_bulk(&pool, &[Person {
        name: String::from("Steven"),
        age: 35,
        nickname: None
    }]);
}
```
