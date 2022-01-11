#! /bin/bash

set -e

AWS=aws

BUCKET=echo-builds

if [ -z "$1" ]; then
    echo "Usage: upload-build.sh {package}"
    exit 1
fi

echo "Uploading build..."
$AWS s3 cp $1 s3://$BUCKET

echo "Done!"
