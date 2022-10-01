#! /bin/bash

set -e

AWS=aws

if [ -z "$1" ]; then
    echo "Usage: logurl.sh {session id}"
    exit 1
fi

$AWS gamelift get-game-session-log-url --game-session-id $1

echo "Done!"
