#!/bin/sh
set -eu
delivery_root=/opt/aio-delivery
git_version=2.43.7
mkdir -p "$delivery_root"
curl --fail --location --retry 3 "https://www.kernel.org/pub/software/scm/git/git-$git_version.tar.xz" -o "$delivery_root/git-$git_version.tar.xz"
printf '%s  %s\n' 657e2374455d9e62f6cdb3e7c55d867b6db5404d744e97e112cc5b0db687a19f "$delivery_root/git-$git_version.tar.xz" | sha256sum -c -
tar -C "$delivery_root" -xf "$delivery_root/git-$git_version.tar.xz"
make -C "$delivery_root/git-$git_version" -j4 prefix="$delivery_root/git" NO_TCLTK=YesPlease NO_GETTEXT=YesPlease CSPRNG_METHOD=openssl install
"$delivery_root/git/bin/git" --version
