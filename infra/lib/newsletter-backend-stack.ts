import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import { RustFunction } from 'cargo-lambda-cdk';

export class NewsletterBackendStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // DynamoDB Table - using free tier capacity
    const subscribersTable = new dynamodb.Table(this, 'SubscribersTable', {
      tableName: 'newsletter_subscribers',
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST, // On-demand capacity, starts in free tier
      removalPolicy: cdk.RemovalPolicy.DESTROY, // For dev/test environments only
    });

    // Add email GSI for looking up subscribers by email
    subscribersTable.addGlobalSecondaryIndex({
      indexName: 'email-index',
      partitionKey: { name: 'email', type: dynamodb.AttributeType.STRING },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // SQS Queue for validating email addresses
    const emailValidationQueue = new cdk.aws_sqs.Queue(this, 'EmailValidationQueue', {
      queueName: 'newsletter-validation-queue',
      visibilityTimeout: cdk.Duration.seconds(30),
      retentionPeriod: cdk.Duration.days(1),
    });

    const subscribeLambda = new RustFunction(this, 'SubscribeLambda', {
      manifestPath: '../Cargo.toml',
      functionName: 'newsletter-subscribe',
      architecture: lambda.Architecture.ARM_64,
      memorySize: 128,

      environment: {
        VALIDATION_QUEUE_URL: emailValidationQueue.queueUrl
      },

      binaryName: 'subscribe',
    });

    // Unsubscribe Lambda Function
    const unsubscribeLambda = new RustFunction(this, 'UnsubscribeLambda', {
      manifestPath: '../Cargo.toml',
      functionName: 'newsletter-unsubscribe',
      architecture: lambda.Architecture.ARM_64,
      memorySize: 128,

      binaryName: 'unsubscribe',
    });

    // Unsubscribe Lambda Function
    const validateLambda = new RustFunction(this, 'ValidateLambda', {
      manifestPath: '../Cargo.toml',
      functionName: 'newsletter-validate',
      architecture: lambda.Architecture.ARM_64,
      memorySize: 128,

      binaryName: 'validate',
    });
    subscribersTable.grantReadWriteData(validateLambda);

    // Confirm Lambda Function
    const confirmLambda = new RustFunction(this, 'ConfirmLambda', {
      manifestPath: '../Cargo.toml',
      functionName: 'newsletter-confirm',
      architecture: lambda.Architecture.ARM_64,
      memorySize: 128,

      binaryName: 'confirm',
    });
    subscribersTable.grantReadWriteData(confirmLambda);

    // Grant Lambda functions permissions to access DynamoDB
    subscribersTable.grantReadWriteData(subscribeLambda);
    subscribersTable.grantReadWriteData(unsubscribeLambda);

    // API Gateway
    const api = new apigateway.RestApi(this, 'NewsletterAPI', {
      restApiName: 'Newsletter Service',
      description: 'API for newsletter subscription management',
      deployOptions: {
        stageName: 'v1',
      },

      // Use minimal configuration to stay within free tier
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
      },
    });

    // Subscribe endpoint
    const subscribeIntegration = new apigateway.LambdaIntegration(subscribeLambda);
    const subscribeResource = api.root.addResource('subscribe');
    subscribeResource.addMethod('POST', subscribeIntegration);

    // Unsubscribe endpoint
    const unsubscribeIntegration = new apigateway.LambdaIntegration(unsubscribeLambda);
    const unsubscribeResource = api.root.addResource('unsubscribe');
    unsubscribeResource.addMethod('POST', unsubscribeIntegration);

    // Confirm endpoint
    const confirmIntegration = new apigateway.LambdaIntegration(confirmLambda);
    const confirmResource = api.root.addResource('confirm');
    confirmResource.addMethod('GET', confirmIntegration);

    emailValidationQueue.grantSendMessages(subscribeLambda);

    // Output the API Gateway URL
    new cdk.CfnOutput(this, 'ApiUrl', {
      value: api.url,
      description: 'The URL of the API Gateway',
    });

    new cdk.CfnOutput(this, 'SqsUrl', {
      value: emailValidationQueue.queueUrl,
      description: 'The URL of the SQS Queue',
    });
  }
}
