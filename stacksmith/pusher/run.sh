#!/bin/bash

# Catch errors and undefined variables
set -euo pipefail

# The directory where the app is installed
readonly installdir=/opt/rusted-iron
readonly conf=env.psuher.list
readonly db=silly-dog-redis-master
# The user that should run the app
readonly system_user=bitnami

cd ${installdir} && ls -al && env && REDISCLOUD_URL=redis://:m67f6rJpcp@${db}:$SILLY_DOG_REDIS_MASTER_SERVICE_PORT REDIS_CONNECTION_MAX_SIZE=256 PORT=8080 RUST_BACKTRACE=full RUST_LOG=info ./target/release/pusher
