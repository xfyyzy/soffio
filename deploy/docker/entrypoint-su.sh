#!/bin/sh
set -euo pipefail

UPLOAD_DIR="${SOFFIO__UPLOADS__DIRECTORY:-/var/lib/soffio/uploads}"

mkdir -p "$UPLOAD_DIR"
chown -R soffio:soffio "$UPLOAD_DIR"

# Drop privileges using busybox su (system user is passwordless).
exec su -s /bin/sh soffio -c "exec \"$@\""
