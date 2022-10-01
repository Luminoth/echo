#! /bin/sh

set -e

AWS=aws

BUCKET=energonsoftware-lambdas

if [ -z "$1" ]; then
    echo "Usage: upload-lambda.sh {lambda}"
    exit 1
fi

echo "Uploading lambda..."
$AWS s3 cp $1 s3://$BUCKET

echo "Done!"
