#! /bin/bash

VERSION="v0.6.0"
OSARCH="linux-amd64"

wget https://github.com/fermyon/spin/releases/download/${VERSION}/spin-${VERSION}-${OSARCH}.tar.gz
tar xfv spin-${VERSION}-${OSARCH}.tar.gz
su
mv ./spin /usr/local/bin/
