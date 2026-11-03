#!/usr/bin/env python3
"""Starling's mod build service (docs/starling-native-mods.md).

A phone sends its native mods (`.hnmod`); this compiles the game with them, in a sandbox, one build at
a time, and hands back the library. Python's standard library only.

    POST /v1/build?version=V   body: for each mod, its length (4 bytes, big-endian) and its file
    GET  /v1/job/ID            {"state": "queued" | "building" | "done" | "failed", ...}
    GET  /v1/job/ID/lib        the library (Range works)
    GET  /v1/busy              {"busy": whether a build is under way or in line}

MOD_BUILD_ROOT (default ~/modbuild) holds versions.json ({"0.1.6": "<commit>"}), base/ (a clean build
cache), source/ (a clone), jobs/ and results/. The same mods for the same version make the same job, so
a second request gets the first one's library.
"""
import hashlib
import http.server
import json
import os
import pathlib
import re
import shutil
import subprocess
import threading
import time
import urllib.parse
import zipfile

ROOT = pathlib.Path(os.environ.get("MOD_BUILD_ROOT", pathlib.Path.home() / "modbuild"))
SANDBOX = pathlib.Path(__file__).with_name("mod_build_sandbox.sh")
PORT = int(os.environ.get("MOD_BUILD_PORT", "8787"))
MAX_BODY = 32 << 20
MAX_MODS = 16
MAX_EXPANDED = 64 << 20
MAX_ENTRIES = 512
MAX_QUEUE = 8
BUILD_SECONDS = 15 * 60
KEEP_RESULTS = 12
ID = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")

lock = threading.Lock()
jobs = {}
line = []
wake = threading.Condition(lock)


class Refused(Exception):
    pass


def allowed(rest):
    """As horizon_mods::index::native::allowed: sources, manifests, data, a crate's text files."""
    parts = rest.split("/")
    name = parts[-1]
    in_data = len(parts) >= 2 and (parts[0] == "data" or (len(parts) >= 3 and parts[1] == "data"))
    text = len(parts) <= 2 and (
        name.startswith(("README", "LICENSE", "LICENCE", "COPYING", "NOTICE")) or name == ".gitignore"
    )
    return rest.endswith((".rs", ".toml")) or in_data or text


def read_bundle(path):
    """The mod's id and the entries of its bundle, checked. Nothing is written or run."""
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as e:
        raise Refused(f"That is not a native mod: {e}")
    with archive:
        infos = [i for i in archive.infolist() if not i.is_dir()]
        if len(infos) > MAX_ENTRIES or sum(i.file_size for i in infos) > MAX_EXPANDED:
            raise Refused("That native mod is too big.")
        for info in infos:
            parts = info.filename.split("/")
            mode = info.external_attr >> 16
            if (
                "\\" in info.filename
                or ":" in info.filename
                or any(p in ("", ".", "..") for p in parts)
                or (mode & 0o170000 and mode & 0o170000 != 0o100000)
            ):
                raise Refused(f"Bad path in the native mod: {info.filename[:80]}")
        try:
            manifest = json.loads(archive.read("horizon-native.json"))
            mod_id = manifest["id"]
        except (KeyError, ValueError, TypeError):
            raise Refused("The native mod has no readable horizon-native.json.")
        if not isinstance(mod_id, str) or not ID.match(mod_id):
            raise Refused("The native mod's id is not valid.")
        prefix = f"mixins/{mod_id}/"
        entries = {}
        for info in infos:
            name = info.filename
            if name == "horizon-native.json" or name == "horizon-mod.toml":
                continue
            if name.startswith(("assets/", "data/")):
                continue
            if not name.startswith(prefix) or not allowed(name[len(prefix) :]):
                raise Refused(f"Unexpected file in the native mod: {name[:80]}")
            entries[name[len(prefix) :]] = archive.read(info)
        if "horizon-mixin.toml" not in entries:
            raise Refused("The native mod has no horizon-mixin.toml.")
        return mod_id, entries


def split_body(body):
    mods, at = [], 0
    while at < len(body):
        if at + 4 > len(body):
            raise Refused("The request was cut short.")
        size = int.from_bytes(body[at : at + 4], "big")
        at += 4
        if size == 0 or at + size > len(body):
            raise Refused("The request was cut short.")
        mods.append(body[at : at + size])
        at += size
    if not mods or len(mods) > MAX_MODS:
        raise Refused("Send from 1 to 16 mods.")
    return mods


def commit_of(version):
    try:
        return json.loads((ROOT / "versions.json").read_text())[version]
    except (OSError, ValueError, KeyError):
        raise Refused(f"The build server has no game for Starling {version[:32]}. Update the game.")


def result_of(key):
    return ROOT / "results" / key


def submit(version, body, client):
    commit = commit_of(version)
    files = sorted(split_body(body))
    digest = hashlib.sha256(f"{version} {commit}".encode())
    for mod in files:
        digest.update(hashlib.sha256(mod).digest())
    key = digest.hexdigest()[:24]
    work = ROOT / "jobs" / key
    done = result_of(key) / "build.json"
    with lock:
        job = jobs.get(key)
        if job and job["state"] in ("queued", "building"):
            return key
        if done.exists():
            jobs[key] = {"state": "done"}
            return key
        if len(line) >= MAX_QUEUE:
            raise Refused("The build server is full. Try again in a few minutes.")
        if any(jobs[k].get("client") == client for k in line):
            raise Refused("You already have a build in line.")
    # Checked and written before it joins the line.
    shutil.rmtree(work, ignore_errors=True)
    (work / "mods").mkdir(parents=True)
    seen = set()
    for i, mod in enumerate(files):
        path = work / f"{i}.hnmod"
        path.write_bytes(mod)
        mod_id, entries = read_bundle(path)
        path.unlink()
        if mod_id in seen:
            raise Refused(f"Two mods are named {mod_id}.")
        seen.add(mod_id)
        for rest, data in entries.items():
            target = work / "mods" / mod_id / rest
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    with lock:
        jobs[key] = {"state": "queued", "commit": commit, "client": client}
        line.append(key)
        wake.notify()
    return key


def why_failed(log):
    """What a player can read of a failed build: the first errors, without the paths."""
    text = log.read_text(errors="replace")
    codes = re.findall(r'"code": "(E_[A-Z_]+)",\s*"message": "([^"]*)"', text)
    if codes:
        return f"The mod does not fit this game ({codes[0][0]}): {codes[0][1]}"[:600]
    errors = [l for l in text.splitlines() if l.startswith("error")]
    if errors:
        lines = [l[:160] for l in errors[:4]]
        return "This mod does not compile on this version of the game. " + " ".join(lines)[:520]
    return "The build failed."


def build(key):
    job = jobs[key]
    work = ROOT / "jobs" / key
    log = work / "build.log"
    out = work / "out"
    out.mkdir(exist_ok=True)
    try:
        with log.open("wb") as sink:
            code = subprocess.run(
                ["bash", str(SANDBOX), str(ROOT), job["commit"], str(work)],
                stdout=sink,
                stderr=subprocess.STDOUT,
                timeout=BUILD_SECONDS,
            ).returncode
    except subprocess.TimeoutExpired:
        return "The build took too long."
    if code != 0 or not (out / "build.json").exists():
        return why_failed(log)
    result = result_of(key)
    shutil.rmtree(result, ignore_errors=True)
    result.mkdir(parents=True)
    shutil.move(out / "libhorizon_android.so", result / "libhorizon_android.so")
    shutil.move(out / "build.json", result / "build.json")
    return None


def forget_old_results():
    folders = [p for p in (ROOT / "results").iterdir() if p.is_dir()]
    folders.sort(key=lambda p: p.stat().st_mtime, reverse=True)
    for old in folders[KEEP_RESULTS:]:
        shutil.rmtree(old, ignore_errors=True)


def worker():
    while True:
        with wake:
            while not line:
                wake.wait()
            key = line[0]
            jobs[key]["state"] = "building"
            jobs[key]["started"] = time.time()
        try:
            error = build(key)
        except Exception as e:  # The line goes on whatever one build does.
            print(f"build {key}: {e!r}", flush=True)
            error = "The build failed."
        with lock:
            line.pop(0)
            if error:
                jobs[key] = {"state": "failed", "error": error}
            else:
                jobs[key] = {"state": "done"}
        shutil.rmtree(ROOT / "jobs" / key, ignore_errors=True)
        forget_old_results()


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "StarlingModBuild"
    timeout = 60

    def answer(self, code, value):
        data = json.dumps(value).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        url = urllib.parse.urlparse(self.path)
        version = urllib.parse.parse_qs(url.query).get("version", [""])[0]
        if url.path != "/v1/build":
            return self.answer(404, {"error": "No such thing here."})
        size = int(self.headers.get("Content-Length") or 0)
        if size <= 0 or size > MAX_BODY:
            return self.answer(413, {"error": "That is too big to compile."})
        body = self.rfile.read(size)
        try:
            # The gateway (mod_build_gateway.py) names the player; only it can reach this port.
            client = self.headers.get("X-Real-IP") or self.client_address[0]
            key = submit(version, body, client)
        except Refused as e:
            return self.answer(400, {"error": str(e)})
        self.answer(200, {"job": key})

    def status(self, key):
        with lock:
            job = dict(jobs.get(key) or {})
            ahead = line.index(key) if key in line else 0
        if not job and (result_of(key) / "build.json").exists():
            job = {"state": "done"}
        state = job.get("state")
        if state == "queued":
            return {"state": state, "ahead": ahead}
        if state == "building":
            return {"state": state, "seconds": int(time.time() - job["started"])}
        if state == "failed":
            return {"state": state, "error": job["error"]}
        if state == "done":
            about = json.loads((result_of(key) / "build.json").read_text())
            return {"state": state, "sha256": about["sha256"], "size": about["size"], "build": about}
        return None

    def do_GET(self):
        parts = urllib.parse.urlparse(self.path).path.strip("/").split("/")
        if parts == ["v1", "busy"]:
            # A `hold` file: someone is working on the machine, and it stays on.
            with lock:
                return self.answer(200, {"busy": bool(line) or (ROOT / "hold").exists()})
        if len(parts) < 3 or parts[:2] != ["v1", "job"] or not re.fullmatch(r"[0-9a-f]{24}", parts[2]):
            return self.answer(404, {"error": "No such thing here."})
        key = parts[2]
        if len(parts) == 3:
            status = self.status(key)
            return self.answer(200 if status else 404, status or {"error": "No such job."})
        path = result_of(key) / "libhorizon_android.so"
        if parts[3:] != ["lib"] or not path.exists():
            return self.answer(404, {"error": "No such thing here."})
        size = path.stat().st_size
        start = 0
        ranged = re.fullmatch(r"bytes=(\d+)-", self.headers.get("Range") or "")
        if ranged and int(ranged.group(1)) < size:
            start = int(ranged.group(1))
        self.send_response(206 if start else 200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(size - start))
        if start:
            self.send_header("Content-Range", f"bytes {start}-{size - 1}/{size}")
        self.end_headers()
        with path.open("rb") as f:
            f.seek(start)
            shutil.copyfileobj(f, self.wfile, 256 * 1024)


def main():
    (ROOT / "jobs").mkdir(parents=True, exist_ok=True)
    (ROOT / "results").mkdir(parents=True, exist_ok=True)
    threading.Thread(target=worker, daemon=True).start()
    http.server.ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
