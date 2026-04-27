use modelite::{BaseModel, Model};

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

    assert_eq!(Student::create_table_sql(), r#"CREATE TABLE IF NOT EXISTS "Student" ("name" TEXT NOT NULL, "age" INTEGER NULL)"#);
}

#[derive(Model)]
#[allow(unused)]
struct Dog {
    pub name: String,
    pub species: Option<String>
}
