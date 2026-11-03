#!/usr/bin/env bash
# One build of the mod service (mod_build_server.py): native_build.sh in a sandbox. A mod's code runs
# while it compiles (build scripts, macros), so that happens with no network, without this machine's
# keys and files, unable to gain privileges, and over a clean build cache it can't change: what the
# build writes stays in a layer of its own, thrown away after, so a mod can't reach a later player's
# build. Namespaces of its own, made with sudo (Ubuntu keeps them from plain users), which it gives up
# before anything of the mod's runs.
#
#   mod_build_sandbox.sh ROOT COMMIT JOB    JOB holds mods/<id>/ and gets out/
#
# MOD_BUILD_BASE=1 builds straight into ROOT/base, to make the clean cache: only for trusted mods.
set -uo pipefail

root=$1
commit=$2
job=$3
seconds=870
base_build=${MOD_BUILD_BASE:-0}

git -C "$root/source" checkout -q --detach "$commit" || exit 1

mods=()
for mod in "$job"/mods/*/; do
  mods+=("$(basename "$mod")")
done

sudo -n timeout --kill-after=10 "$seconds" \
  unshare --net --mount --pid --fork --kill-child --propagation private bash -euc '
home=$1 root=$2 job=$3 uid=$4 gid=$5 base_build=$6
shift 6
mount -t proc proc /proc
mount -t tmpfs -o mode=1777 tmpfs /tmp

# What the build may see, set aside before the home folder goes.
keep=/mnt
mount -t tmpfs tmpfs "$keep"
place() { mkdir -p "$keep/$1"; mount --bind "$2" "$keep/$1"; }
place rustup "$home/.rustup"
place android "$home/android"
place cargo "$root/cargo"
if [ "$base_build" = 1 ]; then
  place cache "$root/base"
else
  layer=$job/overlay
  mkdir -p "$layer/upper" "$layer/work" "$keep/cache"
  chown "$uid:$gid" "$layer/upper"
  mount -t overlay overlay -o "lowerdir=$root/base,upperdir=$layer/upper,workdir=$layer/work" "$keep/cache"
fi
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
exec setpriv --reuid="$uid" --regid="$gid" --clear-groups --no-new-privs \
  --bounding-set=-all --inh-caps=-all \
  env -i HOME="$home" USER="$(id -nu "$uid")" PATH=/usr/bin:/bin CARGO_NET_OFFLINE=true PROFILE=fast \
  NATIVE_LIBRARY_ONLY=1 \
  bash -c '"'"'. "$HOME/.profile"; . "$HOME/.cargo/env"
bundles=()
for mod in "$@"; do bundles+=("$HOME/job/mods/$mod"); done
exec bash "$HOME/source/packaging/android/native_build.sh" "$HOME/source" "$HOME/job/out" "${bundles[@]}"'"'"' \
  sandbox "$@"
' sandbox "$HOME" "$root" "$job" "$(id -u)" "$(id -g)" "$base_build" "${mods[@]}"
code=$?
# The layer holds root's files.
sudo -n rm -rf "$job/overlay"
exit "$code"
