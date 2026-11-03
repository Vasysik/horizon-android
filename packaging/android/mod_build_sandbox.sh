#!/usr/bin/env bash
# One build of the mod service (mod_build_server.py): native_build.sh in a sandbox. A mod's code runs
# while it compiles (build scripts, macros), so that happens with no network, without this machine's
# keys and files, and on a copy of the build cache restored from a clean one, so a mod can't reach a
# later player's build.
#
#   mod_build_sandbox.sh ROOT COMMIT JOB    JOB holds mods/<id>/ and gets out/
set -euo pipefail

root=$1
commit=$2
job=$3
cache=$HOME/.cache/starling-native

# The cache back as it was, by content: a mod could keep a file's time.
rsync -a --delete --checksum "$root/base/" "$root/cache/"
git -C "$root/source" checkout -q --detach "$commit"

bundles=()
for mod in "$job"/mods/*/; do
  bundles+=("/job/mods/$(basename "$mod")")
done

exec bwrap \
  --unshare-all --die-with-parent --new-session \
  --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib64 /lib64 \
  --ro-bind /etc /etc \
  --proc /proc --dev /dev --tmpfs /tmp \
  --tmpfs "$HOME" \
  --ro-bind "$HOME/.profile" "$HOME/.profile" \
  --ro-bind "$HOME/.rustup" "$HOME/.rustup" \
  --ro-bind "$HOME/android" "$HOME/android" \
  --bind "$root/cargo" "$HOME/.cargo" \
  --bind "$root/cache" "$cache" \
  --ro-bind "$root/source" /source \
  --ro-bind "$job/mods" /job/mods \
  --bind "$job/out" /job/out \
  --setenv CARGO_NET_OFFLINE true \
  --setenv PROFILE fast \
  --chdir /tmp \
  bash -c '. "$HOME/.profile"; . "$HOME/.cargo/env"; exec bash /source/packaging/android/native_build.sh /source /job/out "$@"' \
  sandbox "${bundles[@]}"
