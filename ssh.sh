#!/bin/sh
# Credentials and host settings belong in an ignored local wrapper.
set -eu
brick_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
brick_private_ssh=${BEVY_TRIMUI_SSH_WRAPPER:-"$brick_root/ssh.local.sh"}
if [ ! -x "$brick_private_ssh" ]; then
  printf '%s\n' 'Create executable ssh.local.sh (ignored by Git), or set BEVY_TRIMUI_SSH_WRAPPER. See README.md.' >&2
  exit 2
fi
exec "$brick_private_ssh" "$@"
