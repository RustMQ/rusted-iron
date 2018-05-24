#!/bin/bash

# Catch errors and undefined variables
set -euo pipefail

# The directory where the app is installed
readonly installdir=/opt/rusted-iron
# The user that should run the app
readonly system_user=bitnami

# Add bitnami user
useradd ${system_user}

# Typically this is used to install some extra packages
yum install -y wget sudo curl gcc gcc-c++ make openssl-devel

# Rust install
curl https://sh.rustup.rs -sSf | sh -s -- -y
source ~/.profile
source ~/.cargo/env

rustc --version

# Uncompress application in /opt/app
tar xzf ${UPLOADS_DIR}/rustmq.tar.gz -C /opt

# Set permissions
chown -R ${system_user}:${system_user} ${installdir}

# Installing application dependencies
cd /opt && ls -al && cd ${installdir} && cargo build --release -p web
