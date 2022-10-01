#! /bin/sh

set -e

PACKAGE_NAME=echo-notifications.zip

cd ../echo-notifications
cargo lambda build --release

cd target/lambda/echo-notifications
zip -r ../../../../$PACKAGE_NAME .

echo "Built lambda $PACKAGE_NAME"
