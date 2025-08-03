use crate::backend::duckdb::DuckDb;
use crate::backend::KvBackend;
use crate::transaction::{Transaction, TransactionalBackend, TransactionalKvBackend};
use crate::RpcResponse;
use duckdb::{ffi, Connection};
use std::sync::{Arc, Mutex};

/// DuckDB transaction wrapper
#[derive(Debug)]
pub struct DuckDbTransaction {
    conn: Arc<Mutex<Connection>>,
    committed: Arc<Mutex<bool>>,
}

impl DuckDbTransaction {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            committed: Arc::new(Mutex::new(false)),
        }
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }
}

impl Transaction for DuckDbTransaction {
    type Error = duckdb::Error;

    fn commit(self) -> Result<(), Self::Error> {
        let mut committed = self.committed.lock().map_err(|_| {
            duckdb::Error::DuckDBFailure(
                ffi::Error::new(1), // Generic error code
                Some("Failed to acquire lock on committed flag".to_string()),
            )
        })?;
        if *committed {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| {
            duckdb::Error::DuckDBFailure(
                ffi::Error::new(1), // Generic error code
                Some("Failed to acquire lock on connection".to_string()),
            )
        })?;
        conn.execute("COMMIT", [])?;
        *committed = true;
        drop(conn);
        drop(committed);
        Ok(())
    }

    fn rollback(self) -> Result<(), Self::Error> {
        let mut committed = self.committed.lock().map_err(|_| {
            duckdb::Error::DuckDBFailure(
                ffi::Error::new(1), // Generic error code
                Some("Failed to acquire lock on committed flag".to_string()),
            )
        })?;
        if *committed {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| {
            duckdb::Error::DuckDBFailure(
                ffi::Error::new(1), // Generic error code
                Some("Failed to acquire lock on connection".to_string()),
            )
        })?;
        conn.execute("ROLLBACK", [])?;
        *committed = true;
        drop(conn);
        drop(committed);
        Ok(())
    }
}

impl Drop for DuckDbTransaction {
    fn drop(&mut self) {
        if let Ok(committed) = self.committed.lock() {
            if !*committed {
                // Rollback if not explicitly committed
                if let Ok(conn) = self.conn.lock() {
                    let _ = conn.execute("ROLLBACK", []);
                }
            }
        }
    }
}

impl TransactionalBackend for DuckDb {
    type Transaction = DuckDbTransaction;

    fn begin_transaction(&self) -> Result<Self::Transaction, Self::Error> {
        let conn = self.connect_kv()?;
        conn.execute("BEGIN TRANSACTION", [])?;
        Ok(DuckDbTransaction::new(conn))
    }

    fn begin_read_transaction(&self) -> Result<Self::Transaction, Self::Error> {
        // DuckDB doesn't distinguish between read and write transactions
        // at the BEGIN level, so we use the same implementation
        self.begin_transaction()
    }
}

#[async_trait::async_trait]
impl TransactionalKvBackend for DuckDb {
    async fn get_in_transaction(
        &self,
        _transaction_id: &str,
        request: tonic::Streaming<crate::proto::kv::GetRequest>,
    ) -> RpcResponse<Self::GetStream> {
        // For now, we'll use the regular get implementation
        // In a full implementation, we would look up the transaction by ID
        // and use its connection
        self.get(tonic::Request::new(request)).await
    }

    async fn set_in_transaction(
        &self,
        _transaction_id: &str,
        request: tonic::Streaming<crate::proto::kv::SetRequest>,
    ) -> RpcResponse<Self::SetStream> {
        // For now, we'll use the regular set implementation
        self.set(tonic::Request::new(request)).await
    }

    async fn delete_in_transaction(
        &self,
        _transaction_id: &str,
        request: tonic::Streaming<crate::proto::kv::DeleteRequest>,
    ) -> RpcResponse<Self::DeleteStream> {
        // For now, we'll use the regular delete implementation
        self.delete(tonic::Request::new(request)).await
    }
}
