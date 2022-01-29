#!/bin/bash

set -e
set -o pipefail

# Skript creates a custom-signed certificate
# Parameter1 = name of the cert

CERT_KEY_NAME=$1
CERT_FILE=$CERT_KEY_NAME.crt

SKIP=false

if test -f "$CERT_FILE"; then
  EXPIRE_DATE=$(openssl x509 -in $CERT_FILE -noout -enddate |sed 's/notAfter\=//')
  EXPIRE_SECS=`date -d "${EXPIRE_DATE}" +%s`
  EXPIRE_TIME=$(( ${EXPIRE_SECS} - `date +%s` ))
  DAYS=$(( ${EXPIRE_TIME} / 24 / 3600 ))
  echo Certificate $CERT_FILE is valid for $DAYS days
  if [ $DAYS -gt 10 ]; then
    SKIP=true
  fi;
fi;

if ! $SKIP; then
  export CERT_CN=localhost

  echo Prepare Signing-Request-Config from Template
  cat signing-request.config.template | envsubst >> src.txt

  echo Generate Private-Key and Certificate-Signing-Request for $CERT_KEY_NAME
  openssl req \
      -new \
      -nodes \
      -config src.txt \
      -keyout ${CERT_KEY_NAME}.pem \
      -out ${CERT_KEY_NAME}.sr

  echo Generate an OpenSSL Certificate for $CERT_KEY_NAME
  openssl x509 -req \
      -in ${CERT_KEY_NAME}.sr \
      -extensions v3_req \
      -extfile src.txt \
      -CA customCA.crt -CAkey customCA.key \
      -CAcreateserial \
      -CAserial customCA.srl \
      -out $CERT_FILE \
      -passin file:passphrase.txt \
      -days 200

  echo Cleaning up temporary files
  rm src.txt
  rm ${CERT_KEY_NAME}.sr

  echo DONE
else
  echo Certificate $CERT_FILE exists and is still valid. Nothing to do.
fi;

