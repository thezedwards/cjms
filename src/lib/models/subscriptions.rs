use serde_json::Value as JsonValue;
use sqlx::{query, query_as, Error, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::status_history::{Status, UpdateStatus};

use super::status_history::DateRange;

// All the public fields of Subscription for clean construction
pub struct PartialSubscription {
    pub id: Uuid,
    pub flow_id: String,
    pub subscription_id: String,
    pub report_timestamp: OffsetDateTime,
    pub subscription_created: OffsetDateTime,
    // Note this is a hash
    pub fxa_uid: String,
    pub quantity: i32,
    pub plan_id: String,
    pub plan_currency: String,
    pub plan_amount: i32,
    pub country: Option<String>,
    pub coupons: Option<String>,
    pub aic_id: Option<Uuid>,
    pub aic_expires: Option<OffsetDateTime>,
    pub cj_event_value: Option<String>,
}

#[derive(Debug)]
pub struct Subscription {
    pub id: Uuid,
    pub flow_id: String,
    pub subscription_id: String,
    pub report_timestamp: OffsetDateTime,
    pub subscription_created: OffsetDateTime,
    // Note this is a hash
    pub fxa_uid: String,
    pub quantity: i32,
    pub plan_id: String,
    pub plan_currency: String,
    pub plan_amount: i32,
    pub country: Option<String>,
    pub coupons: Option<String>,
    pub aic_id: Option<Uuid>,
    pub aic_expires: Option<OffsetDateTime>,
    pub cj_event_value: Option<String>,
    // Note we use strings and json, not enums, in the database for simplicity
    status: Option<String>,
    status_t: Option<OffsetDateTime>,
    status_history: Option<JsonValue>,
}
impl PartialEq for Subscription {
    fn eq(&self, other: &Self) -> bool {
        let simple_match = self.id == other.id &&
        self.flow_id == other.flow_id &&
        self.subscription_id == other.subscription_id &&
        // When timestamps go in and out of database they lose precision to milliseconds
        self.report_timestamp.unix_timestamp() == other.report_timestamp.unix_timestamp() &&
        self.subscription_created.unix_timestamp() == other.subscription_created.unix_timestamp() &&
        self.fxa_uid == other.fxa_uid &&
        self.quantity == other.quantity &&
        self.plan_id == other.plan_id &&
        self.plan_currency == other.plan_currency &&
        self.plan_amount == other.plan_amount &&
        self.country == other.country &&
        self.coupons == other.coupons &&
        self.aic_id == other.aic_id &&
        self.cj_event_value == other.cj_event_value &&
        self.status == other.status
        // Compare manually if needed
        // self.status_history == other.status_history
        ;
        let aic_expires_match = match self.aic_expires {
            Some(self_v) => match other.aic_expires {
                Some(other_v) => self_v.unix_timestamp() == other_v.unix_timestamp(),
                None => false,
            },
            None => other.aic_expires.is_none(),
        };
        let status_t_match = match self.status_t {
            Some(self_v) => match other.status_t {
                Some(other_v) => self_v.unix_timestamp() == other_v.unix_timestamp(),
                None => false,
            },
            None => other.status_t.is_none(),
        };
        status_t_match && aic_expires_match && simple_match
    }
}
impl Eq for Subscription {}

impl UpdateStatus for Subscription {
    fn get_raw_status(&self) -> Option<String> {
        self.status.clone()
    }

    fn get_raw_status_history(&self) -> Option<JsonValue> {
        self.status_history.clone()
    }

    fn set_raw_status(&mut self, v: Option<String>) {
        self.status = v;
    }

    fn set_raw_status_history(&mut self, v: Option<JsonValue>) {
        self.status_history = v;
    }

    fn get_status_t(&self) -> Option<OffsetDateTime> {
        self.status_t
    }

    fn set_status_t(&mut self, v: Option<OffsetDateTime>) {
        self.status_t = v;
    }
}

impl Subscription {
    pub fn new(partial_sub: PartialSubscription) -> Self {
        let mut sub = Subscription {
            id: partial_sub.id,
            flow_id: partial_sub.flow_id,
            subscription_id: partial_sub.subscription_id,
            report_timestamp: partial_sub.report_timestamp,
            subscription_created: partial_sub.subscription_created,
            fxa_uid: partial_sub.fxa_uid,
            quantity: partial_sub.quantity,
            plan_id: partial_sub.plan_id,
            plan_currency: partial_sub.plan_currency,
            plan_amount: partial_sub.plan_amount,
            country: partial_sub.country,
            coupons: partial_sub.coupons,
            aic_id: partial_sub.aic_id,
            aic_expires: partial_sub.aic_expires,
            cj_event_value: partial_sub.cj_event_value,
            status: None,
            status_t: None,
            status_history: None,
        };
        sub.update_status(Status::NotReported);
        sub
    }
}

pub struct SubscriptionModel<'a> {
    pub db_pool: &'a PgPool,
}

impl SubscriptionModel<'_> {
    pub async fn create_from_sub(&self, sub: &Subscription) -> Result<Subscription, Error> {
        query_as!(
            Subscription,
            "INSERT INTO subscriptions (
                id,
                flow_id,
                subscription_id,
                report_timestamp,
                subscription_created,
                fxa_uid,
                quantity,
                plan_id,
                plan_currency,
                plan_amount,
                country,
                coupons,
                aic_id,
                aic_expires,
                cj_event_value,
                status,
                status_t,
                status_history
             )
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
			RETURNING *",
            sub.id,
            sub.flow_id,
            sub.subscription_id,
            sub.report_timestamp,
            sub.subscription_created,
            sub.fxa_uid,
            sub.quantity,
            sub.plan_id,
            sub.plan_currency,
            sub.plan_amount,
            sub.country,
            sub.coupons,
            sub.aic_id,
            sub.aic_expires,
            sub.cj_event_value,
            sub.status,
            sub.status_t,
            sub.status_history,
        )
        .fetch_one(self.db_pool)
        .await
    }

    pub async fn fetch_one_by_id(&self, id: &Uuid) -> Result<Subscription, Error> {
        query_as!(
            Subscription,
            "SELECT * FROM subscriptions WHERE id = $1",
            id
        )
        .fetch_one(self.db_pool)
        .await
    }

    pub async fn fetch_one_by_flow_id(&self, flow_id: &str) -> Result<Subscription, Error> {
        query_as!(
            Subscription,
            "SELECT * FROM subscriptions WHERE flow_id = $1",
            flow_id
        )
        .fetch_one(self.db_pool)
        .await
    }

    pub async fn fetch_one_by_subscription_id(
        &self,
        subscription_id: &str,
    ) -> Result<Subscription, Error> {
        query_as!(
            Subscription,
            "SELECT * FROM subscriptions WHERE subscription_id = $1",
            subscription_id
        )
        .fetch_one(self.db_pool)
        .await
    }

    pub async fn fetch_all(&self) -> Result<Vec<Subscription>, Error> {
        query_as!(Subscription, "SELECT * FROM subscriptions")
            .fetch_all(self.db_pool)
            .await
    }

    pub async fn fetch_all_by_status(&self, status: Status) -> Result<Vec<Subscription>, Error> {
        // Note that users of this function rely on status_t being available
        query_as!(
            Subscription,
            r#"
            SELECT *
            FROM subscriptions
            WHERE status = $1
            AND status_t IS NOT NULL"#,
            status.to_string()
        )
        .fetch_all(self.db_pool)
        .await
    }

    pub async fn update_sub_status(
        &self,
        id: &Uuid,
        new_status: Status,
    ) -> Result<Subscription, Error> {
        let mut sub = self.fetch_one_by_id(id).await?;
        sub.update_status(new_status);
        query_as!(
            Subscription,
            r#"UPDATE subscriptions
            SET
                status = $1,
                status_t = $2,
                status_history = $3
            WHERE id = $4
			RETURNING *"#,
            sub.status,
            sub.status_t,
            sub.status_history,
            id,
        )
        .fetch_one(self.db_pool)
        .await
    }

    pub async fn get_reported_date_range(&self) -> Result<DateRange, Error> {
        let result = query!(
            "SELECT MIN(status_t), MAX(status_t) FROM subscriptions WHERE status = 'Reported' AND status_t IS NOT NULL",
        )
        .fetch_one(self.db_pool)
        .await?;
        Ok(DateRange {
            min: result.min,
            max: result.max,
        })
    }
}

#[cfg(test)]
pub mod test_subscriptions {

    use super::*;
    use crate::test_utils::{
        random_ascii_string, random_currency_or_country, random_price, random_simple_ascii_string,
    };
    use time::Duration;

    pub fn make_fake_sub() -> Subscription {
        Subscription::new(PartialSubscription {
            id: Uuid::new_v4(),
            flow_id: random_simple_ascii_string(),
            subscription_id: random_simple_ascii_string(),
            report_timestamp: OffsetDateTime::now_utc(),
            subscription_created: OffsetDateTime::now_utc() - Duration::hours(35),
            fxa_uid: random_ascii_string(),
            quantity: 1,
            plan_id: random_simple_ascii_string(),
            plan_currency: random_currency_or_country(),
            plan_amount: random_price(),
            country: Some(random_currency_or_country()),
            coupons: Some(random_ascii_string()),
            aic_id: Some(Uuid::new_v4()),
            aic_expires: Some(OffsetDateTime::now_utc()),
            cj_event_value: Some(random_ascii_string()),
        })
    }

    #[test]
    fn test_new_sets_not_reported_status_and_history() {
        let new = Subscription::new(PartialSubscription {
            id: Uuid::new_v4(),
            flow_id: random_simple_ascii_string(),
            subscription_id: random_simple_ascii_string(),
            report_timestamp: OffsetDateTime::now_utc(),
            subscription_created: OffsetDateTime::now_utc(),
            fxa_uid: random_simple_ascii_string(),
            quantity: 1,
            plan_id: random_simple_ascii_string(),
            plan_currency: random_simple_ascii_string(),
            plan_amount: 1,
            country: None,
            coupons: None,
            aic_id: None,
            aic_expires: None,
            cj_event_value: None,
        });
        let now = OffsetDateTime::now_utc();
        assert_eq!(new.get_status().unwrap(), Status::NotReported);
        assert_eq!(
            new.get_status_t().unwrap().unix_timestamp(),
            now.unix_timestamp()
        );
        let status_history = new.get_status_history().unwrap();
        assert_eq!(status_history.entries.len(), 1);
        assert_eq!(status_history.entries[0].status, Status::NotReported);
        assert_eq!(
            status_history.entries[0].t.unix_timestamp(),
            now.unix_timestamp()
        );
    }
}
