#! /bin/bash

set -e

VERSION=`git rev-parse --short HEAD`
PACKAGE_NAME=echo-$VERSION.zip

rm -rf build/
rm -f echo*.zip

mkdir build

cp install.sh build

cargo install --target x86_64-unknown-linux-musl --path . --root build/
rm build/.crates*

cd build
zip -r ../$PACKAGE_NAME .

cd ..
rm -rf build

echo "Built package $PACKAGE_NAME"
