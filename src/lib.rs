use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Configuration constants
pub const TABLE_NAME: &str = "newsletter_subscribers";

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscriber {
    pub id: String,
    pub email: String,
    pub active: bool,
    pub validated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Subscriber {
    pub fn new(email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            active: true,
            validated: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn to_dynamodb_item(&self) -> HashMap<String, AttributeValue> {
        let mut item = HashMap::new();

        item.insert("id".to_string(), AttributeValue::S(self.id.clone()));
        item.insert("email".to_string(), AttributeValue::S(self.email.clone()));
        item.insert("active".to_string(), AttributeValue::Bool(self.active));
        item.insert(
            "validated".to_string(),
            AttributeValue::Bool(self.validated),
        );
        item.insert(
            "created_at".to_string(),
            AttributeValue::S(self.created_at.to_rfc3339()),
        );
        item.insert(
            "updated_at".to_string(),
            AttributeValue::S(self.updated_at.to_rfc3339()),
        );

        item
    }

    pub fn from_dynamodb_item(item: &HashMap<String, AttributeValue>) -> Option<Self> {
        let id = item.get("id")?.as_s().ok()?;
        let email = item.get("email")?.as_s().ok()?;
        let active = item.get("active")?.as_bool().ok()?;
        let validated = item.get("validated")?.as_bool().ok()?;
        let created_at = DateTime::parse_from_rfc3339(item.get("created_at")?.as_s().ok()?)
            .ok()?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(item.get("updated_at")?.as_s().ok()?)
            .ok()?
            .with_timezone(&Utc);

        Some(Self {
            id: id.clone(),
            email: email.clone(),
            active: *active,
            validated: *validated,
            created_at,
            updated_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

// Helper function to create an API response
pub fn create_response(
    status_code: u16,
    body: ApiResponse,
) -> lambda_http::Response<lambda_http::Body> {
    lambda_http::Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(lambda_http::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap()
}
