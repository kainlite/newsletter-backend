use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_sqs::Client as SqsClient;
use email_address::*;
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use newsletter_backend::{ApiResponse, SubscribeRequest, Subscriber, TABLE_NAME, create_response};
use serde_json::json;
use std::env;
use tracing::info;

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Get the SQS queue URL from environment variables
    let queue_url = match env::var("VALIDATION_QUEUE_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("VALIDATION_QUEUE_URL not set in environment");
            // Fallback to a default URL for development or provide an error response
            "https://sqs.us-east-1.amazonaws.com/000000000000/newsletter-validation-queue"
                .to_string()
        }
    };

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

    let subscribe_request: SubscribeRequest = match serde_json::from_str(body) {
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

    // Validate email (basic validation)
    if !EmailAddress::is_valid(&subscribe_request.email) {
        return Ok(create_response(
            400,
            ApiResponse {
                success: false,
                message: "Invalid email format".to_string(),
            },
        ));
    }

    // Create subscriber
    let subscriber = Subscriber::new(subscribe_request.email.clone());

    // Initialize AWS SDK
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let dynamodb_client = Client::new(&config);

    // Initialize SQS client with the same config
    let sqs_client = SqsClient::new(&config);

    // Check if email already exists (to avoid duplicates)
    let email_query = match dynamodb_client
        .query()
        .table_name(TABLE_NAME)
        .index_name("email-index")
        .key_condition_expression("email = :email")
        .expression_attribute_values(
            ":email",
            aws_sdk_dynamodb::types::AttributeValue::S(subscribe_request.email.clone()),
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
                    aws_sdk_dynamodb::types::AttributeValue::S(subscribe_request.email.clone()),
                )
                .send()
                .await;
            Err(scan_result)
        }
    };

    match email_query {
        Ok(result) => {
            if let Some(items) = result.items() {
                if !items.is_empty() {
                    // Email already exists
                    return Ok(create_response(
                        200,
                        ApiResponse {
                            success: true,
                            message: "Email is already subscribed".to_string(),
                        },
                    ));
                }
            }
        }
        Err(err) => {
            info!("Error checking for existing email: {:?}", err);
            // Continue with subscription even if query fails
        }
    }

    // Put item in DynamoDB
    let put_result = dynamodb_client
        .put_item()
        .table_name(TABLE_NAME)
        .set_item(Some(subscriber.to_dynamodb_item()))
        .send()
        .await;

    match put_result {
        Ok(_) => {
            // Send validation message to SQS
            let message = json!({
                "action": "validate_email",
                "email": subscribe_request.email,
                "subscriber_id": subscriber.id
            })
            .to_string();

            // Send message to SQS queue
            match sqs_client
                .send_message()
                .queue_url(&queue_url)
                .message_body(message)
                .send()
                .await
            {
                Ok(_) => info!("Sent validation message to queue"),
                Err(e) => info!("Failed to send validation message to queue: {:?}", e),
            };

            Ok(create_response(
                201,
                ApiResponse {
                    success: true,
                    message: "Successfully subscribed. Validation email will be sent shortly."
                        .to_string(),
                },
            ))
        }
        Err(err) => {
            info!("Error adding subscriber: {:?}", err);
            Ok(create_response(
                500,
                ApiResponse {
                    success: false,
                    message: "Failed to subscribe".to_string(),
                },
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
