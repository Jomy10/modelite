use modelite::{BaseModel, Model};
#[cfg(feature = "sqlx")]
use sqlx::prelude::FromRow;

#[cfg(feature = "sqlx")]
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use tokio::runtime;

    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(future)
}

#[derive(BaseModel)]
#[allow(unused)]
struct Student {
    pub name: String,
    pub age: Option<u32>,
}

#[test]
fn test_basemodel() {
    assert_eq!(Student::table_name(), "Student");
    assert_eq!(Student::COLUMNS, ["name", "age"]);

    assert_eq!(Student::create_table_sql(), r#"create table if not exists "Student" ("name" TEXT NOT NULL, "age" INTEGER NULL)"#);
}

#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[derive(Model, Debug, PartialEq)]
struct Dog {
    pub name: String,
    pub species: Option<String>
}

#[cfg(feature = "sqlx")]
#[test]
fn test_execute_all() {
    use sqlx::Connection;

    block_on(async {
        let mut conn = sqlx::SqliteConnection::connect(":memory:").await?;
        let dog = Dog { name: "Good boy".to_string(), species: None };
        Dog::drop_table().execute(&mut conn).await?;
        Dog::create_table().execute(&mut conn).await?;
        println!("{}", Dog::insert_sql());
        let query = Dog::insert_bulk(&mut conn, vec![&dog]).await?;
        modelite::execute_all!(query, &mut conn).await?;

        let docs: Vec<Dog> = Dog::select_all().fetch_all(&mut conn).await?;
        assert_eq!(docs, vec![dog]);

        Ok::<(), sqlx::Error>(())
    }).unwrap();
}


#[derive(Model)]
#[unique(country, place, street, house_number)]
struct House {
    country: String,
    place: String,
    street: String,
    house_number: String,
}

#[cfg(feature = "sqlx")]
#[test]
fn test_unique() {
    block_on(async {
        let conn = sqlx::SqlitePool::connect(":memory:").await?;
        let house1 = House { country: "BE".to_string(), place: "Luxemburg".to_string(), street: "la rue".to_string(), house_number: "96A".to_string() };
        let house2 = House { country: "BE".to_string(), place: "Luxemburg".to_string(), street: "la rue".to_string(), house_number: "96B".to_string() };

        House::create_table().execute(&conn).await?;

        House::insert(&house1).execute(&conn).await?;
        House::insert(&house2).execute(&conn).await?;
        assert!(
            House::insert(&house1).execute(&conn).await.is_err()
        );

        Ok::<(), sqlx::Error>(())
    }).unwrap();
}
