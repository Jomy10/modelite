pub trait BaseModel: Sized {
    const COLUMNS: &'static [&'static str];

    fn table_name() -> &'static str {
        std::any::type_name::<Self>()
            .split("::")
            .last()
            .unwrap()
    }

    fn create_table_sql() -> &'static str;
    fn insert_sql() -> &'static str;
    fn insert_one_sql() -> &'static str;
    fn select_sql() -> &'static str;
    fn drop_table_sql() -> &'static str;
}

pub mod util {
    use std::borrow::Cow;
    use std::iter::repeat_n;

    use crate::BaseModel;

    #[macro_export]
    macro_rules! cache {
        ($x:expr) => {{
            static CACHE: OnceLock<usize> = OnceLock::new();
            *CACHE.get_or_init(|| $x)
        }};
    }

    pub fn create_table_sql<M: BaseModel>(data: Cow<'static, str>) -> String {
        format!("create table if not exists \"{}\" ({})",
            M::table_name(),
            data
        )
    }

    pub fn insert_sql<M: BaseModel>() -> String {
        format!("insert into \"{}\" ({})",
            M::table_name(),
            M::COLUMNS.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(",")
        )
    }

    pub fn insert_one_sql<M: BaseModel>() -> String {
        format!("{} values ({})",
            insert_sql::<M>(),
            repeat_n("?", M::COLUMNS.len()).collect::<Vec<_>>().join(",")
        )
    }

    pub fn select_sql<M: BaseModel>() -> String {
        format!("select {} from \"{}\"",
            M::COLUMNS.iter().map(|c| format!("\"{}\".\"{}\"", M::table_name(), c)).collect::<Vec<_>>().join(","),
            M::table_name()
        )
    }

    pub fn drop_table_sql<M: BaseModel>() -> String {
        format!("drop table if exists \"{}\"", M::table_name())
    }
}
