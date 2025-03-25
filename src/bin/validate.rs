use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Duration, Utc};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use newsletter_backend::{Subscriber, TABLE_NAME};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SqsEvent {
    #[serde(rename = "Records")]
    records: Vec<SqsRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SqsRecord {
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "body")]
    body: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationMessage {
    action: String,
    email: String,
    subscriber_id: String,
}

async fn function_handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize AWS SDK
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let dynamodb_client = Client::new(&config);

    info!("Processing {} SQS records", event.payload.records.len());

    for record in event.payload.records {
        // Parse the message from the SQS record
        let message_result: Result<ValidationMessage, serde_json::Error> =
            serde_json::from_str(&record.body);

        match message_result {
            Ok(message) => {
                info!("Processing validation for email: {}", message.email);

                // Generate a validation token with UUID
                let token = Uuid::new_v4().to_string();

                // Calculate expiration (24 hours from now)
                let expiration = Utc::now() + Duration::hours(24);

                // Store the token in DynamoDB
                let update_result = dynamodb_client
                    .update_item()
                    .table_name(TABLE_NAME)
                    .key("id", aws_sdk_dynamodb::types::AttributeValue::S(message.subscriber_id.clone()))
                    .update_expression("SET validation_token = :token, token_expiration = :expiration, updated_at = :updated_at")
                    .expression_attribute_values(":token", AttributeValue::S(token.clone()))
                    .expression_attribute_values(":expiration", AttributeValue::S(expiration.to_rfc3339()))
                    .expression_attribute_values(":updated_at", AttributeValue::S(Utc::now().to_rfc3339()))
                    .send()
                    .await;

                match update_result {
                    Ok(_) => {
                        // Generate the validation URL that would be included in the email
                        let validation_url = format!(
                            "https://yourfrontend.com/validate?id={}&token={}",
                            message.subscriber_id, token
                        );

                        info!("Generated validation URL: {}", validation_url);

                        // In a real application, you would send an email with the validation URL
                        // For demo purposes, we'll just log the URL and simulate the email sending
                        info!(
                            "Simulated email sent to: {} with validation URL",
                            message.email
                        );
                    }
                    Err(e) => info!("Error storing validation token: {:?}", e),
                }
            }
            Err(e) => info!("Error parsing SQS message: {:?}", e),
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
