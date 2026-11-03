#!/usr/bin/env bash
# One build of the mod service (mod_build_server.py): native_build.sh in a sandbox. A mod's code runs
# while it compiles (build scripts, macros), so that happens with no network, without this machine's
# keys and files, unable to gain privileges, and on a copy of the build cache restored from a clean
# one, so a mod can't reach a later player's build. Namespaces of its own, made with sudo (Ubuntu keeps
# them from plain users), which it gives up before anything of the mod's runs.
#
#   mod_build_sandbox.sh ROOT COMMIT JOB    JOB holds mods/<id>/ and gets out/
set -euo pipefail

root=$1
commit=$2
job=$3
seconds=870

# The cache back as it was, by content: a mod could keep a file's time.
rsync -a --delete --checksum "$root/base/" "$root/cache/"
git -C "$root/source" checkout -q --detach "$commit"

mods=()
for mod in "$job"/mods/*/; do
  mods+=("$(basename "$mod")")
done

exec sudo -n timeout --kill-after=10 "$seconds" \
  unshare --net --mount --pid --fork --kill-child --propagation private bash -euc '
home=$1 root=$2 job=$3 uid=$4 gid=$5
shift 5
mount -t proc proc /proc
mount -t tmpfs -o mode=1777 tmpfs /tmp

# What the build may see, set aside before the home folder goes.
keep=/mnt
mount -t tmpfs tmpfs "$keep"
place() { mkdir -p "$keep/$1"; mount --bind "$2" "$keep/$1"; }
place rustup "$home/.rustup"
place android "$home/android"
place cargo "$root/cargo"
place cache "$root/cache"
place source "$root/source"
place mods "$job/mods"
place out "$job/out"
cp "$home/.profile" "$keep/profile"

mount -t tmpfs -o "mode=755,uid=$uid,gid=$gid" tmpfs "$home"
back() {
  mkdir -p "$home/$2"
  mount --move "$keep/$1" "$home/$2"
  [ "$3" = rw ] || mount -o remount,bind,ro "$home/$2"
}
back rustup .rustup ro
back android android ro
back cargo .cargo rw
back cache .cache/starling-native rw
back source source ro
back mods job/mods ro
back out job/out rw
cp "$keep/profile" "$home/.profile"
chown -R "$uid:$gid" "$home/.profile"
chown "$uid:$gid" "$home" "$home/.cache" "$home/job"
umount "$keep"

cd /tmp
exec setpriv --reuid="$uid" --regid="$gid" --init-groups --no-new-privs \
  --bounding-set=-all --inh-caps=-all \
  env -i HOME="$home" USER="$(id -nu "$uid")" PATH=/usr/bin:/bin CARGO_NET_OFFLINE=true PROFILE=fast \
  bash -c '"'"'. "$HOME/.profile"; . "$HOME/.cargo/env"
bundles=()
for mod in "$@"; do bundles+=("$HOME/job/mods/$mod"); done
exec bash "$HOME/source/packaging/android/native_build.sh" "$HOME/source" "$HOME/job/out" "${bundles[@]}"'"'"' \
  sandbox "$@"
' sandbox "$HOME" "$root" "$job" "$(id -u)" "$(id -g)" "${mods[@]}"
