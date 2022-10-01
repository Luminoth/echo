#! /bin/sh

set -e

AWS=aws

BUCKET=energonsoftware-lambdas

if [ -z "$1" ]; then
    echo "Usage: deploy-lambda.sh {lambda}"
    exit 1
fi

echo "Deploying lambda..."
$AWS lambda update-function-code --function-name $1 --s3-bucket $BUCKET --s3-key $1.zip

echo "Done!"
