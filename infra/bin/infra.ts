#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { NewsletterBackendStack } from '../lib/newsletter-backend-stack';

const app = new cdk.App();
new NewsletterBackendStack(app, 'NewsletterBackendStack', {
  /* If you don't specify 'env', this stack will be environment-agnostic.
   * For more information, see https://docs.aws.amazon.com/cdk/latest/guide/environments.html */
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION || 'us-east-1' // Default to us-east-1 if not specified
  },
});

