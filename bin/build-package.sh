#! /bin/bash

set -e

VERSION=`git rev-parse --short HEAD`
PACKAGE_NAME=echo-$VERSION.zip

cd ..
rm -rf build/
rm -f  echo*.zip

mkdir build

cp echo/install.sh build/
chmod 755 echo/install.sh

cd echo
cargo install --target x86_64-unknown-linux-musl --path . --root ../build/
cd -
rm build/.crates*

cd build
zip -r ../$PACKAGE_NAME .

cd ..
rm -rf build

echo "Built package $PACKAGE_NAME"
