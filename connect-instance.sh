#! /bin/bash

set -e

AWS=aws
PYTHON=python3

if [ -z "$2" ]; then
    echo "Usage: logurl.sh {fleet id} {instance id}"
    exit 1
fi

ip=`dig TXT +short o-o.myaddr.l.google.com @ns1.google.com | tr -d \"`

echo "Opening ports for connection from '$ip'..."
$AWS gamelift update-fleet-port-settings --fleet-id $1 --inbound-permission-authorizations "FromPort=22,ToPort=22,IpRange=$ip/32,Protocol=TCP"

echo
echo "**IMPORTANT** When finished, please run '$AWS gamelift update-fleet-port-settings --fleet-id $1 --inbound-permission-revocations \"FromPort=22,ToPort=22,IpRange=$ip/32,Protocol=TCP\"'"
echo

echo "Getting instance access ..."
res=`$AWS gamelift get-instance-access --fleet-id $1 --instance-id $2`
$PYTHON -c "import json; print(json.load('$res')['UserName'])"

echo "Done!"
