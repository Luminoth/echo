#! /bin/sh

set -e

AWS=aws

if [ -z "$1" ]; then
    echo "Usage: publish-lambda.sh {lambda}"
    exit 1
fi

echo "Publishing lambda..."
$AWS lambda publish-version --function-name $1

echo "Done!"
