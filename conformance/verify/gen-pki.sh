#!/usr/bin/env bash
# Generate the Spike B test PKI under conformance/verify/pki/.
#
#   legit Root CA  -> author cert, filler cert, filler-revoked cert (+ CRL revoking it)
#   untrusted CA   -> untrusted-author cert (for the not-whitelisted case)
#
# All keys RSA-2048, signatures SHA-256. OpenSSL only; no secrets of value (test fixtures).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PKI="$HERE/pki"
rm -rf "$PKI"; mkdir -p "$PKI/newcerts"
cd "$PKI"
: > index.txt
echo 1000 > serial
echo 1000 > crlnumber

cat > ca.cnf <<'CNF'
[ca]
default_ca = CA_default
[CA_default]
dir = .
database = $dir/index.txt
new_certs_dir = $dir/newcerts
certificate = $dir/ca.crt
private_key = $dir/ca.key
serial = $dir/serial
crlnumber = $dir/crlnumber
default_md = sha256
default_days = 825
default_crl_days = 30
policy = policy_any
x509_extensions = usr_cert
[policy_any]
commonName = supplied
organizationName = optional
[usr_cert]
basicConstraints = CA:FALSE
[req]
distinguished_name = dn
[dn]
CNF

q() { "$@" >/dev/null 2>&1; }

# --- legit Root CA ---
q openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.crt -days 3650 \
  -subj "/CN=HFP Test Root CA" -sha256 -addext "basicConstraints=critical,CA:TRUE"

issue() { # name subject
  local name="$1" subj="$2"
  q openssl req -newkey rsa:2048 -nodes -keyout "$name.key" -out "$name.csr" -subj "$subj" -sha256
  q openssl ca -batch -config ca.cnf -in "$name.csr" -out "$name.crt"
}

issue author "/CN=ACME Service s.r.o./O=ACME"
issue filler "/CN=Jan Novak/O=ACME"
issue filler-revoked "/CN=Mallory/O=ACME"

# Revoke filler-revoked and publish a CRL.
q openssl ca -config ca.cnf -revoke filler-revoked.crt
q openssl ca -config ca.cnf -gencrl -out crl.pem
q openssl crl -in crl.pem -outform DER -out crl.der

# --- untrusted CA + an author cert under it ---
q openssl req -x509 -newkey rsa:2048 -nodes -keyout untrusted-ca.key -out untrusted-ca.crt \
  -days 3650 -subj "/CN=Rogue CA" -sha256 -addext "basicConstraints=critical,CA:TRUE"
q openssl req -newkey rsa:2048 -nodes -keyout untrusted-author.key -out untrusted-author.csr \
  -subj "/CN=ACME Service s.r.o./O=ACME" -sha256
q openssl x509 -req -in untrusted-author.csr -CA untrusted-ca.crt -CAkey untrusted-ca.key \
  -CAcreateserial -out untrusted-author.crt -days 825 -sha256 2>/dev/null

# DER trust anchors + thumbprints.
openssl x509 -in ca.crt -outform DER -out ca.der
openssl x509 -in untrusted-ca.crt -outform DER -out untrusted-ca.der
sha256() { openssl dgst -sha256 "$1" | awk '{print $2}'; }
{
  echo "ca=$(sha256 ca.der)"
  echo "untrusted-ca=$(sha256 untrusted-ca.der)"
} > thumbprints.txt

echo "PKI ready in $PKI"
cat thumbprints.txt
