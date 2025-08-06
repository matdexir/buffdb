mod sqlite {
    type Backend = buffdb::backend::Sqlite;
    const BLOB_PATH: &str = "blob_store.sqlite-test.db";
    const KV_PATH: &str = "kv_store.sqlite-test.db";

    mod blob {
        include!("blob.rs");
    }
    mod kv {
        include!("kv.rs");
    }
    mod query {
        include!("query.rs");
    }
    // Transaction tests removed - gRPC transaction support is deprecated
    mod index {
        include!("index_test.rs");
    }
}

// DuckDB tests temporarily disabled until backend is stabilized
// #[cfg(feature = "duckdb")]
// mod duckdb {
//     type Backend = buffdb::backend::DuckDb;
//     const BLOB_PATH: &str = "blob_store.duckdb-test.db";
//     const KV_PATH: &str = "kv_store.duckdb-test.db";
//
//     mod blob {
//         include!("blob.rs");
//     }
//     mod kv {
//         include!("kv.rs");
//     }
//     mod query {
//         include!("query.rs");
//     }
// }

mod helpers;

#[cfg(rust_analyzer)]
mod blob;
#[cfg(rust_analyzer)]
mod index_test;
#[cfg(rust_analyzer)]
mod kv;
#[cfg(rust_analyzer)]
mod query;
