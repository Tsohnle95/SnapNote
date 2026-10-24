#!/bin/zsh
# Stable local code-signing identity for SnapNote development builds.
#
# macOS TCC binds Screen Recording grants to the app's designated requirement.
# Ad-hoc signatures (`codesign --sign -`) pin that requirement to the binary's
# cdhash, so every rebuild looks like a brand-new app and macOS asks again.
# Signing with a self-signed certificate pins it to the certificate instead, so
# one grant survives all subsequent rebuilds.
set -eu
cd "${0:A:h:h}"
identity="${1:-${SNAPNOTE_SIGNING_IDENTITY:-SnapNote Local Signing}}"
keychain="$HOME/Library/Keychains/login.keychain-db"

if security find-identity -v -p codesigning "$keychain" 2>/dev/null |
    grep -F "\"$identity\"" >/dev/null; then
    print "Code signing identity ready: $identity"
    exit 0
fi

print "Creating one-time code signing identity: $identity"
print "This is a local development identity. It never leaves this Mac."
workdir="$(mktemp -d "${TMPDIR:-/tmp}/snapnote-signing.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT
cat > "$workdir/openssl.cnf" <<EOF
[ req ]
distinguished_name = dn
x509_extensions = ext
prompt = no
[ dn ]
CN = $identity
O = SnapNote Local Development
[ ext ]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
subjectKeyIdentifier = hash
EOF
openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$workdir/key.pem" -out "$workdir/cert.pem" \
    -days 3650 -config "$workdir/openssl.cnf" >/dev/null 2>&1
openssl pkcs12 -export -out "$workdir/identity.p12" \
    -inkey "$workdir/key.pem" -in "$workdir/cert.pem" \
    -passout pass:snapnote -name "$identity" >/dev/null 2>&1
security import "$workdir/identity.p12" -k "$keychain" -P snapnote \
    -T /usr/bin/codesign >/dev/null
# Mark the certificate as trusted for code signing in the user's trust store.
security add-trusted-cert -r trustRoot -p codeSign -k "$keychain" \
    "$workdir/cert.pem" >/dev/null

if ! security find-identity -v -p codesigning "$keychain" 2>/dev/null |
    grep -F "\"$identity\"" >/dev/null; then
    print -u2 "Could not create '$identity'."
    print -u2 "Open Keychain Access → Certificate Assistant → Create a Certificate,"
    print -u2 "choose Code Signing, name it '$identity', then run this script again."
    exit 1
fi
print "Created and trusted $identity"
