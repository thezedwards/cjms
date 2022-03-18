use crate::utils::get_db_pool;
use lib::models::aic::AICArchiveModel;

use super::aic::test_aic;
use uuid::Uuid;

#[tokio::test]
async fn test_aic_archive_model_fetch_one_by_ids() {
    let db_pool = get_db_pool().await;
    let model = AICArchiveModel { db_pool: &db_pool };
    let created = model
        .create_from_aic(&test_aic())
        .await
        .expect("Failed to create test object.");
    // id
    let result = model
        .fetch_one_by_id(&created.id)
        .await
        .expect("Could not fetch from DB.");
    assert_eq!(result, created);
    // flow id
    let result = model
        .fetch_one_by_flow_id(&created.flow_id)
        .await
        .expect("Could not fetch from DB.");
    assert_eq!(result, created);
}

#[tokio::test]
async fn test_aic_archive_model_fetch_one_by_uuid_if_not_available() {
    let db_pool = get_db_pool().await;
    let model = AICArchiveModel { db_pool: &db_pool };
    model
        .create_from_aic(&test_aic())
        .await
        .expect("Failed to create test object.");
    let bad_id = Uuid::new_v4();
    // id
    let result = model.fetch_one_by_id(&bad_id).await;
    match result {
        Err(sqlx::Error::RowNotFound) => {
            println!("Success");
        }
        _ => {
            panic!("This should not have happened.");
        }
    };
    // flow id
    let result = model.fetch_one_by_flow_id("bad_id").await;
    match result {
        Err(sqlx::Error::RowNotFound) => {
            println!("Success");
        }
        _ => {
            panic!("This should not have happened.");
        }
    };
}

#[tokio::test]
async fn test_aic_archive_model_create_by_aic() {
    let db_pool = get_db_pool().await;
    let model = AICArchiveModel { db_pool: &db_pool };
    let aic = test_aic();
    model
        .create_from_aic(&aic)
        .await
        .expect("Failed to create test object.");
    let result = model
        .fetch_one_by_id(&aic.id)
        .await
        .expect("Could not fetch from DB.");
    assert_eq!(result, aic);
}