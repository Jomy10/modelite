macro_rules! moduse {
    ($mod: ident) => {
        mod $mod;
        pub use $mod::*;
    };
    ($mod: ident, $feature: literal) => {
        #[cfg(feature = $feature)]
        mod $mod;
        pub use $mod::*;
    };
}

moduse!(model);
moduse!(query, "sqlx");
moduse!(handle);

pub use modelite_macros::*;
