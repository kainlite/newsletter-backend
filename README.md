# Newsletter Backend API

A serverless newsletter subscription backend built with Rust, AWS Lambda, and DynamoDB, designed to stay within the AWS free tier.

## Features

- **Subscribe API**: Adds new email addresses to DynamoDB
- **Unsubscribe API**: Marks email addresses as inactive
- **Serverless Architecture**: Uses AWS Lambda and API Gateway
- **Free Tier Compatible**: Configured to use AWS services within the free tier limits

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [AWS CDK](https://docs.aws.amazon.com/cdk/latest/guide/getting_started.html)
- [Node.js and npm](https://nodejs.org/)
- [AWS CLI](https://aws.amazon.com/cli/) configured with appropriate credentials
- [cargo-lambda](https://github.com/cargo-lambda/cargo-lambda)
- [cargo-lambda-cdk](https://github.com/cargo-lambda/cargo-lambda-cdk)

## Project Structure

```
newsletter-backend/
├── src/
│   ├── bin/
│   │   ├── subscribe.rs      # Lambda function for subscribing
│   │   └── unsubscribe.rs    # Lambda function for unsubscribing
│   └── lib.rs                # Shared code for Lambda functions
├── infra/                    # CDK infrastructure code
│   ├── bin/
│   │   └── infra.ts          # CDK app entry point
│   └── lib/
│       └── newsletter-backend-stack.ts # CDK stack definition
├── Cargo.toml                # Rust dependencies
└── README.md                 # This file
```

## Building and Deployment

### 1. Build the Lambda functions

```bash
# In the project root
cargo lambda build --release --arm64
```

### 2. Deploy the infrastructure

```bash
# Navigate to the infra directory
cd infra

# Install dependencies
npm install

# Bootstrap CDK (if you haven't already)
npx cdk bootstrap

# Deploy
npx cdk deploy
```

The CDK deployment will output the API Gateway URL for your API.

## API Endpoints

### Subscribe

**Endpoint**: `POST /subscribe`

**Request Body**:
```json
{
  "email": "user@example.com"
}
```

**Response**:
```json
{
  "success": true,
  "message": "Successfully subscribed"
}
```

### Unsubscribe

**Endpoint**: `POST /unsubscribe`

**Request Body**:
```json
{
  "email": "user@example.com"
}
```

**Response**:
```json
{
  "success": true,
  "message": "Successfully unsubscribed"
}
```

## AWS Free Tier Considerations

This project is designed to stay within the AWS Free Tier limits:

- **DynamoDB**: Uses on-demand capacity which includes 25 WCUs and 25 RCUs per month in the free tier
- **Lambda**: Configured with minimal memory (128MB) and the free tier includes 1 million requests per month
- **API Gateway**: Free tier includes 1 million API calls per month
- **CloudWatch Logs**: There may be some minimal costs for extensive logging

Monitor your AWS billing dashboard to ensure you stay within free tier limits.

## Customization

- Modify `TABLE_NAME` in `lib.rs` if you want to change the DynamoDB table name
- Adjust the validation logic in the Lambda functions for stricter email validation
- Add additional fields to the `Subscriber` struct if needed

## Cleaning Up

To avoid any potential charges, you can delete all resources when you're done:

```bash
cd infra
npx cdk destroy
```

This will remove all AWS resources created by this project.
