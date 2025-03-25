use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use newsletter_backend::{ApiResponse, TABLE_NAME, create_response};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct ConfirmRequest {
    id: String,
    token: String,
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse query parameters
    let query_params = event.uri().query().unwrap_or("");
    let params: Vec<(String, String)> = query_params
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                _ => None,
            }
        })
        .collect();

    // Extract id and token from query parameters
    let id = params
        .iter()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.clone());

    let token = params
        .iter()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.clone());

    // Check if id and token are provided
    let (id, token) = match (id, token) {
        (Some(id), Some(token)) => (id, token),
        _ => {
            return Ok(create_response(
                400,
                ApiResponse {
                    success: false,
                    message: "Missing id or token".to_string(),
                },
            ));
        }
    };

    // Initialize AWS SDK
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let dynamodb_client = Client::new(&config);

    // Get the subscriber from DynamoDB
    let get_result = dynamodb_client
        .get_item()
        .table_name(TABLE_NAME)
        .key("id", AttributeValue::S(id.clone()))
        .send()
        .await;

    match get_result {
        Ok(result) => {
            if let Some(item) = result.item() {
                // Check if the subscriber has a validation token
                if let Some(validation_token) = item.get("validation_token") {
                    if let Ok(stored_token) = validation_token.as_s() {
                        // Check if the token matches
                        if stored_token == &token {
                            // Check if the token is expired
                            if let Some(expiration) = item.get("token_expiration") {
                                if let Ok(expiration_str) = expiration.as_s() {
                                    if let Ok(expiration_time) =
                                        DateTime::parse_from_rfc3339(expiration_str)
                                    {
                                        let now = Utc::now();

                                        if now < expiration_time.with_timezone(&Utc) {
                                            // Token is valid, mark the subscriber as validated
                                            let update_result = dynamodb_client
                                                .update_item()
                                                .table_name(TABLE_NAME)
                                                .key("id", AttributeValue::S(id.clone()))
                                                .update_expression("SET validated = :validated, updated_at = :updated_at REMOVE validation_token, token_expiration")
                                                .expression_attribute_values(":validated", AttributeValue::Bool(true))
                                                .expression_attribute_values(":updated_at", AttributeValue::S(Utc::now().to_rfc3339()))
                                                .send()
                                                .await;

                                            match update_result {
                                                Ok(_) => {
                                                    // Return a success response
                                                    return Ok(create_response(
                                                        200,
                                                        ApiResponse {
                                                            success: true,
                                                            message: "Email successfully validated"
                                                                .to_string(),
                                                        },
                                                    ));
                                                }
                                                Err(e) => {
                                                    info!(
                                                        "Error updating validation status: {:?}",
                                                        e
                                                    );
                                                    return Ok(create_response(
                                                        500,
                                                        ApiResponse {
                                                            success: false,
                                                            message: "Failed to validate email"
                                                                .to_string(),
                                                        },
                                                    ));
                                                }
                                            }
                                        } else {
                                            // Token is expired
                                            return Ok(create_response(
                                                400,
                                                ApiResponse {
                                                    success: false,
                                                    message: "Validation token has expired"
                                                        .to_string(),
                                                },
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If we get here, the token was invalid or not found
                return Ok(create_response(
                    400,
                    ApiResponse {
                        success: false,
                        message: "Invalid validation token".to_string(),
                    },
                ));
            } else {
                // Subscriber not found
                return Ok(create_response(
                    404,
                    ApiResponse {
                        success: false,
                        message: "Subscriber not found".to_string(),
                    },
                ));
            }
        }
        Err(e) => {
            info!("Error getting subscriber: {:?}", e);
            return Ok(create_response(
                500,
                ApiResponse {
                    success: false,
                    message: "Failed to retrieve subscriber information".to_string(),
                },
            ));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
