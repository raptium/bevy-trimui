#!/bin/sh
# Credentials and host settings belong in an ignored local wrapper.
set -eu
boat_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
boat_private_ssh=${BOAT_SSH_WRAPPER:-"$boat_root/ssh.local.sh"}
if [ ! -x "$boat_private_ssh" ]; then
  printf '%s\n' 'Create executable ssh.local.sh (ignored by Git), or set BOAT_SSH_WRAPPER. See tools/device/README.md.' >&2
  exit 2
fi
exec "$boat_private_ssh" "$@"
