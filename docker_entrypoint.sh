#!/bin/sh

set -euo pipefail

cd /root
export HOST_IP=$(ip -4 route list match 0/0 | awk '{print $3}')

exec cups
