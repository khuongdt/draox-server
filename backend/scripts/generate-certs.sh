#!/usr/bin/env bash
# Generate self-signed TLS certificates for local development.
# Output: backend/certs/server.crt and backend/certs/server.key
# Do NOT use these in production — get a certificate from a trusted CA.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="$SCRIPT_DIR/../certs"
DAYS=365
SUBJECT="/CN=localhost/O=Draox-Dev/C=US"

mkdir -p "$CERTS_DIR"

openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$CERTS_DIR/server.key" \
    -out    "$CERTS_DIR/server.crt" \
    -days   "$DAYS"               \
    -subj   "$SUBJECT"            \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

echo "Certificates written to $CERTS_DIR"
echo "  cert: $CERTS_DIR/server.crt"
echo "  key:  $CERTS_DIR/server.key"
echo ""
echo "These are self-signed dev certificates and will show a browser warning."
echo "For production, replace them with certificates from a trusted CA."
