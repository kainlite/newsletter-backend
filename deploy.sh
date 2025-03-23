#!/bin/bash
set -e

./build.sh

echo "Deploying CDK infrastructure..."
cd infra
npm install
npx cdk deploy

echo "Deployment complete!"
echo "Try testing the API with:"
echo 'curl -v URL/v1/subscribe -H "Content-Type: application/json" -X POST -d '{ "email": "sample@email.com" }'
