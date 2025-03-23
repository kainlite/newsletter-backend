use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use chrono::Utc;
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use newsletter_backend::{ApiResponse, TABLE_NAME, UnsubscribeRequest, create_response};
use tracing::info;

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse request body
    let body = match event.body() {
        Body::Text(text) => text,
        _ => {
            return Ok(create_response(
                400,
                ApiResponse {
                    success: false,
                    message: "Invalid request body".to_string(),
                },
            ));
        }
    };

    let unsubscribe_request: UnsubscribeRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(_) => {
            return Ok(create_response(
                400,
                ApiResponse {
                    success: false,
                    message: "Invalid JSON format".to_string(),
                },
            ));
        }
    };

    // Initialize AWS SDK
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let dynamodb_client = Client::new(&config);

    // Find the subscriber by email
    let query_result = match dynamodb_client
        .query()
        .table_name(TABLE_NAME)
        .index_name("email-index")
        .key_condition_expression("email = :email")
        .expression_attribute_values(
            ":email",
            AttributeValue::S(unsubscribe_request.email.clone()),
        )
        .send()
        .await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            info!("Error querying by email index: {:?}", err);
            // If the index isn't ready yet, we'll do a scan as a fallback
            let scan_result = dynamodb_client
                .scan()
                .table_name(TABLE_NAME)
                .filter_expression("email = :email")
                .expression_attribute_values(
                    ":email",
                    AttributeValue::S(unsubscribe_request.email.clone()),
                )
                .send()
                .await;
            Err(scan_result)
        }
    };

    match query_result {
        Ok(output) => {
            if let Some(items) = output.items() {
                if items.is_empty() {
                    return Ok(create_response(
                        404,
                        ApiResponse {
                            success: false,
                            message: "Email not found in subscribers".to_string(),
                        },
                    ));
                }

                // Get the first match (should be only one)
                if let Some(item) = items.first() {
                    if let Some(id) = item.get("id") {
                        if let Ok(id_str) = id.as_s() {
                            // Update the subscriber to inactive
                            let update_result = dynamodb_client
                                .update_item()
                                .table_name(TABLE_NAME)
                                .key("id", AttributeValue::S(id_str.clone()))
                                .update_expression("SET active = :active, updated_at = :updated_at")
                                .expression_attribute_values(":active", AttributeValue::Bool(false))
                                .expression_attribute_values(
                                    ":updated_at",
                                    AttributeValue::S(Utc::now().to_rfc3339()),
                                )
                                .send()
                                .await;

                            match update_result {
                                Ok(_) => {
                                    return Ok(create_response(
                                        200,
                                        ApiResponse {
                                            success: true,
                                            message: "Successfully unsubscribed".to_string(),
                                        },
                                    ));
                                }
                                Err(err) => {
                                    info!("Error updating subscriber: {:?}", err);
                                    return Ok(create_response(
                                        500,
                                        ApiResponse {
                                            success: false,
                                            message: "Failed to unsubscribe".to_string(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            Ok(create_response(
                404,
                ApiResponse {
                    success: false,
                    message: "Subscriber not found".to_string(),
                },
            ))
        }
        Err(err) => {
            info!("Error querying DynamoDB: {:?}", err);
            Ok(create_response(
                500,
                ApiResponse {
                    success: false,
                    message: "Error processing unsubscribe request".to_string(),
                },
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
