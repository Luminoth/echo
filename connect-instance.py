#! /usr/bin/python3

import os
import stat
import sys

import boto3
import requests

if len(sys.argv) < 3:
    print('Usage: connect-instance {fleet id} {instance id}')
    sys.exit(1)

fleet_id = sys.argv[1]
instance_id = sys.argv[2]
ext_ip = requests.get('https://checkip.amazonaws.com').text.strip()

print("Opening ssh on {}:{} for connection from {} ...".format(fleet_id, instance_id, ext_ip))

client = boto3.client('gamelift')
client.update_fleet_port_settings(
    FleetId=fleet_id,
    InboundPermissionAuthorizations=[
        {
            'FromPort': 22,
            'ToPort': 22,
            'IpRange': '{}/32'.format(ext_ip),
            'Protocol': 'TCP'
        },
    ]
)

print('\n**IMPORTANT** When finished, please run `aws gamelift update-fleet-port-settings --fleet-id {} --inbound-permission-revocations \"FromPort=22,ToPort=22,IpRange={}/32,Protocol=TCP\"`\n'.format(fleet_id, ext_ip))

print("Getting instance access ...")
res = client.get_instance_access(
    FleetId=fleet_id,
    InstanceId=instance_id
)

instance_ip = res['InstanceAccess']['IpAddress']
credentials = res['InstanceAccess']['Credentials']
username = credentials['UserName']
secret = credentials['Secret']

pem_file_name = '{}-{}.pem'.format(fleet_id, instance_id)

with open(pem_file_name, 'w') as f:
    f.write(secret)
os.chmod(pem_file_name, stat.S_IRUSR | stat.S_IWUSR)

# TODO: this would be a lot cooler if we just ran ssh from here
# and then cleaned up the port settings and PEM file ourselves
print('`ssh -i {} {}@{}`'.format(pem_file_name, username, instance_ip))
