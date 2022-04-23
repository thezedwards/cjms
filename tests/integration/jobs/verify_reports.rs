use crate::{models::subscriptions::make_fake_sub, utils::get_test_db_pool};
use lib::{
    cj::client::CJClient,
    jobs::verify_reports::verify_reports_with_cj,
    models::{
        status_history::{Status, StatusHistoryEntry, UpdateStatus},
        subscriptions::SubscriptionModel,
    },
    settings::get_settings,
    telemetry::StatsD,
};
use serde_json::json;
use time::{Duration, OffsetDateTime};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_correct_and_incorrectly_received_subscriptions_are_handled_correctly() {
    // SETUP
    let settings = get_settings();
    let mock_statsd = StatsD::new(&settings);
    let db_pool = get_test_db_pool().await;
    let sub_model = SubscriptionModel { db_pool: &db_pool };

    // Sub 1 - Reported, expect to have been recieved by CJ
    let mut sub_1 = make_fake_sub();
    sub_1.update_status(Status::Reported);
    // Sub 2 - Reported 48 hours ago, CJ has the wrong amount
    let mut sub_2 = make_fake_sub();
    sub_2.update_status(Status::Reported);
    sub_2.set_status_t(Some(sub_2.get_status_t().unwrap() - Duration::hours(48)));
    // Sub 3 - Reported 48 hours ago, CJ has the wrong sku
    let mut sub_3 = make_fake_sub();
    sub_3.update_status(Status::Reported);
    sub_3.set_status_t(Some(sub_3.get_status_t().unwrap() - Duration::hours(48)));
    // Sub 4 - Reported 48 hours ago, CJ has the wrong id
    let mut sub_4 = make_fake_sub();
    sub_4.update_status(Status::Reported);
    sub_4.set_status_t(Some(sub_4.get_status_t().unwrap() - Duration::hours(48)));
    // Sub 5 - Reported < 36 hours ago, CJ has the wrong id
    let mut sub_5 = make_fake_sub();
    sub_5.update_status(Status::Reported);
    sub_5.set_status_t(Some(sub_5.get_status_t().unwrap() - Duration::hours(35)));

    for sub in [&sub_1, &sub_2, &sub_3, &sub_4, &sub_5] {
        sub_model
            .create_from_sub(sub)
            .await
            .expect("Failed to create sub.");
    }
    let mock_cj = MockServer::start().await;
    let required_query = r#"{
advertiserCommissions(
    forAdvertisers: ["6002970"],
    sincePostingDate:"2022-04-22T00:00:00Z",
    beforePostingDate:"2022-05-22T00:00:00Z",
) {
    count
    records {
        original
        orderId
        postingDate
        correctionReason
        saleAmountPubCurrency
        items {
            sku
        }
    }
}}"#;
    let response_body = json!(
        {"data":
            {"advertiserCommissions":
                {
                    "count": 5,
                    "records": [
                        {
                            "original": true,
                            "orderId": sub_1.id,
                            "saleAmountPubCurrency": sub_1.plan_amount,
                            "items": [
                                {
                                    "sku": sub_1.plan_id
                                }
                            ]
                        },
                        {
                            "original": true,
                            "orderId": sub_2.id,
                            "saleAmountPubCurrency": -999,
                            "items": [
                                {
                                    "sku": sub_2.plan_id
                                }
                            ]
                        },
                        {
                            "original": true,
                            "orderId": sub_3.id,
                            "saleAmountPubCurrency": sub_3.plan_amount,
                            "items": [
                                {
                                    "sku": "WRONG SKU"
                                }
                            ]
                        },
                        {
                            "original": true,
                            "orderId": "WRONGID",
                            "saleAmountPubCurrency": sub_4.plan_amount,
                            "items": [
                                {
                                    "sku": sub_4.plan_id
                                }
                            ]
                        },
                        {
                            "original": true,
                            "orderId": "WRONGID",
                            "saleAmountPubCurrency": sub_5.plan_amount,
                            "items": [
                                {
                                    "sku": sub_5.plan_id
                                }
                            ]
                        },
                    ]
                }
            }
        }
    );
    let response = ResponseTemplate::new(200).set_body_json(response_body);
    Mock::given(path("/"))
        .and(method("GET"))
        .and(header(
            "Authorization",
            format!("Bearer {}", settings.cj_api_access_token).as_str(),
        ))
        .and(body_json(&json!({ "query": required_query })))
        .respond_with(response)
        .expect(1)
        .mount(&mock_cj)
        .await;
    let mock_cj_client = CJClient::new(&settings, None, Some(&mock_cj.uri()));

    // GO
    let now = OffsetDateTime::now_utc();
    verify_reports_with_cj(&db_pool, &mock_cj_client, &mock_statsd).await;

    // ASSERT
    let sub_1_updated = sub_model
        .fetch_one_by_id(&sub_1.id)
        .await
        .expect("Could not get sub");
    let sub_2_updated = sub_model
        .fetch_one_by_id(&sub_2.id)
        .await
        .expect("Could not get sub");
    let sub_3_updated = sub_model
        .fetch_one_by_id(&sub_3.id)
        .await
        .expect("Could not get sub");
    let sub_4_updated = sub_model
        .fetch_one_by_id(&sub_4.id)
        .await
        .expect("Could not get sub");
    let sub_5_updated = sub_model
        .fetch_one_by_id(&sub_5.id)
        .await
        .expect("Could not get sub");

    assert_eq!(sub_1_updated.get_status().unwrap(), Status::CJReceived);
    let updated_history = sub_1_updated.get_status_history().unwrap();
    assert_eq!(updated_history.entries.len(), 2);
    assert_eq!(
        updated_history.entries[1],
        StatusHistoryEntry {
            status: Status::CJReceived,
            t: now
        }
    );
    for not_found_sub in [&sub_2_updated, &sub_3_updated, &sub_4_updated] {
        println!("Testing sub: {}", not_found_sub.id);
        assert_eq!(not_found_sub.get_status().unwrap(), Status::CJNotReceived);
        let updated_history = not_found_sub.get_status_history().unwrap();
        assert_eq!(updated_history.entries.len(), 2);
        assert_eq!(
            updated_history.entries[1],
            StatusHistoryEntry {
                status: Status::CJNotReceived,
                t: now
            }
        );
    }
    // Leave unchanged as we'll try again to see if the report comes through
    assert_eq!(sub_5_updated.get_status().unwrap(), Status::Reported);
    let updated_history = sub_5_updated.get_status_history().unwrap();
    assert_eq!(updated_history.entries.len(), 1);
}