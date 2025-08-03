use anyhow::Result;
use buffdb::proto::kv::{
    BeginTransactionRequest, CommitTransactionRequest, GetRequest, RollbackTransactionRequest,
    SetRequest,
};
use buffdb::transitive::kv_client;
use buffdb::Location;
use futures::{stream, StreamExt as _};
use serial_test::serial;
use std::sync::LazyLock;

static KV_STORE_LOC: LazyLock<Location> = LazyLock::new(|| Location::InMemory);

#[tokio::test]
#[serial]
async fn test_basic_transaction_commit() -> Result<()> {
    let mut client = kv_client::<_, super::Backend>(KV_STORE_LOC.clone()).await?;

    // Begin transaction
    let begin_resp = client
        .begin_transaction(BeginTransactionRequest {
            read_only: Some(false),
            timeout_ms: Some(5000),
        })
        .await?;

    let transaction_id = begin_resp.into_inner().transaction_id;

    // Set a value within the transaction
    let mut set_resp = client
        .set(stream::iter([SetRequest {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
            transaction_id: Some(transaction_id.clone()),
        }]))
        .await?
        .into_inner();

    // Verify set response
    if let Some(resp) = set_resp.next().await {
        assert_eq!(resp?.key, "test_key");
    }

    // Commit transaction
    let commit_resp = client
        .commit_transaction(CommitTransactionRequest {
            transaction_id: transaction_id.clone(),
        })
        .await?;

    assert!(commit_resp.into_inner().success);

    // Verify the value persists after commit
    let mut get_resp = client
        .get(stream::iter([GetRequest {
            key: "test_key".to_string(),
            transaction_id: None,
        }]))
        .await?
        .into_inner();

    if let Some(resp) = get_resp.next().await {
        assert_eq!(resp?.value, "test_value");
    }

    drop(client);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_rollback() -> Result<()> {
    let mut client = kv_client::<_, super::Backend>(KV_STORE_LOC.clone()).await?;

    // Begin transaction
    let begin_resp = client
        .begin_transaction(BeginTransactionRequest {
            read_only: Some(false),
            timeout_ms: Some(5000),
        })
        .await?;

    let transaction_id = begin_resp.into_inner().transaction_id;

    // Set a value within the transaction
    let _set_resp = client
        .set(stream::iter([SetRequest {
            key: "rollback_key".to_string(),
            value: "rollback_value".to_string(),
            transaction_id: Some(transaction_id.clone()),
        }]))
        .await?;

    // Rollback transaction
    let rollback_resp = client
        .rollback_transaction(RollbackTransactionRequest {
            transaction_id: transaction_id.clone(),
        })
        .await?;

    assert!(rollback_resp.into_inner().success);

    // Verify the value does not exist after rollback
    let mut get_resp = client
        .get(stream::iter([GetRequest {
            key: "rollback_key".to_string(),
            transaction_id: None,
        }]))
        .await?
        .into_inner();

    // Should get an error or no value
    match get_resp.next().await {
        Some(Err(_)) => {} // Expected - key not found
        Some(Ok(_)) => panic!("Value should not exist after rollback"),
        None => {} // Also acceptable - no results
    }

    drop(client);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_read_only_transaction() -> Result<()> {
    let mut client = kv_client::<_, super::Backend>(KV_STORE_LOC.clone()).await?;

    // First, set some initial data
    let _set_resp = client
        .set(stream::iter([SetRequest {
            key: "readonly_test".to_string(),
            value: "initial_value".to_string(),
            transaction_id: None,
        }]))
        .await?;

    // Begin read-only transaction
    let begin_resp = client
        .begin_transaction(BeginTransactionRequest {
            read_only: Some(true),
            timeout_ms: Some(5000),
        })
        .await?;

    let transaction_id = begin_resp.into_inner().transaction_id;

    // Read value within transaction
    let mut get_resp = client
        .get(stream::iter([GetRequest {
            key: "readonly_test".to_string(),
            transaction_id: Some(transaction_id.clone()),
        }]))
        .await?
        .into_inner();

    if let Some(resp) = get_resp.next().await {
        assert_eq!(resp?.value, "initial_value");
    }

    // Commit read-only transaction
    let commit_resp = client
        .commit_transaction(CommitTransactionRequest { transaction_id })
        .await?;

    assert!(commit_resp.into_inner().success);

    drop(client);
    Ok(())
}
