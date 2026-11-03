#!/usr/bin/env python3
"""The door to Starling's mod build service (mod_build_server.py), on a small machine that stays on.

The build machine costs money by the hour, so it sleeps: a request wakes it, and it is put back to sleep
once it has had nothing to do for a while. Behind Caddy, which holds the HTTPS. The machine's own identity
may only start, stop and read that one VM. Python's standard library only.

    BUILD_BACKEND  http://host:8787
    BUILD_VM       the VM's Azure resource id
    GATEWAY_PORT   default 8788
"""
import http.client
import http.server
import json
import os
import shutil
import threading
import time
import urllib.parse
import urllib.request

BACKEND = urllib.parse.urlparse(os.environ["BUILD_BACKEND"])
VM = os.environ["BUILD_VM"]
PORT = int(os.environ.get("GATEWAY_PORT", "8788"))
ARM = "https://management.azure.com"
IDENTITY = (
    "http://169.254.169.254/metadata/identity/oauth2/token"
    "?api-version=2018-02-01&resource=https%3A%2F%2Fmanagement.azure.com%2F"
)
MAX_BODY = 32 << 20
# Nothing to do for this long, and the machine goes back to sleep.
IDLE_SECONDS = 15 * 60
# Started and never answered: put back to sleep rather than left on.
START_SECONDS = 20 * 60
# Between asks to start it, while it boots.
START_AGAIN_SECONDS = 90
# Builds a day, so one player or everyone together can't run the bill up.
PER_ADDRESS = 8
PER_DAY = 80
DAY = 24 * 3600

lock = threading.Lock()
last_activity = time.time()
started_at = None
asked_at = 0.0
accepted = []


def power(action):
    """Asks Azure to `start` or `deallocate` the VM. False when it would not."""
    try:
        request = urllib.request.Request(IDENTITY, headers={"Metadata": "true"})
        with urllib.request.urlopen(request, timeout=10) as answer:
            token = json.load(answer)["access_token"]
        request = urllib.request.Request(
            f"{ARM}{VM}/{action}?api-version=2024-07-01",
            data=b"",
            method="POST",
            headers={"Authorization": f"Bearer {token}"},
        )
        with urllib.request.urlopen(request, timeout=30):
            pass
        print(f"asked Azure to {action}", flush=True)
        return True
    except OSError as e:
        print(f"{action} failed: {e}", flush=True)
        return False


def connection(timeout):
    """To the service, connected: a sleeping machine doesn't refuse, it says nothing, so the connecting
    has a short wait of its own."""
    c = http.client.HTTPConnection(BACKEND.hostname, BACKEND.port, timeout=4)
    c.connect()
    c.sock.settimeout(timeout)
    return c


def backend_status():
    """The service's `busy`, or None while it does not answer."""
    try:
        c = connection(4)
        c.request("GET", "/v1/busy")
        answer = c.getresponse()
        return json.loads(answer.read()).get("busy") if answer.status == 200 else None
    except (OSError, ValueError, http.client.HTTPException):
        return None


def wake():
    """Whether the service answers; if not, it is being woken."""
    global started_at, asked_at
    if backend_status() is not None:
        return True
    with lock:
        ask = time.time() - asked_at > START_AGAIN_SECONDS
        if ask:
            asked_at = time.time()
    if ask and power("start"):
        with lock:
            if started_at is None:
                started_at = time.time()
    return False


def sleeper():
    """Puts back to sleep what this started, when it has been idle. A machine someone else started
    is left alone, and so is one whose service answered and then stopped: someone is working on it,
    or it is already off."""
    global started_at
    answered = False
    while True:
        time.sleep(60)
        with lock:
            started, active = started_at, last_activity
        if started is None:
            answered = False
            continue
        now = time.time()
        busy = backend_status()
        if busy is None:
            if answered:
                with lock:
                    started_at = None
                continue
            give_up = now - started > START_SECONDS
        else:
            answered = True
            give_up = not busy and now - active > IDLE_SECONDS and now - started > IDLE_SECONDS
        if give_up and power("deallocate"):
            with lock:
                started_at = None


def within_quota(address, job):
    """Whether this build counts within the day's. A job already counted costs nothing."""
    now = time.time()
    with lock:
        accepted[:] = [a for a in accepted if now - a[0] < DAY]
        if any(a[1] == address and a[2] == job for a in accepted):
            return True
        if (
            sum(a[1] == address for a in accepted) >= PER_ADDRESS
            or len(accepted) >= PER_DAY
        ):
            return False
        accepted.append((now, address, job))
    return True


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "StarlingModGateway"
    timeout = 120

    def address(self):
        forwarded = self.headers.get("X-Forwarded-For") or self.client_address[0]
        return forwarded.split(",")[0].strip()

    def answer(self, code, value):
        data = json.dumps(value).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def seen(self):
        global last_activity
        with lock:
            last_activity = time.time()

    def do_POST(self):
        self.seen()
        url = urllib.parse.urlparse(self.path)
        if url.path != "/v1/build":
            return self.answer(404, {"error": "No such thing here."})
        size = int(self.headers.get("Content-Length") or 0)
        if size <= 0 or size > MAX_BODY:
            return self.answer(413, {"error": "That is too big to compile."})
        body = self.rfile.read(size)
        if not wake():
            return self.answer(200, {"starting": True})
        try:
            c = connection(60)
            c.request(
                "POST",
                self.path,
                body,
                {"Content-Type": "application/octet-stream", "X-Real-IP": self.address()},
            )
            response = c.getresponse()
            data = response.read()
        except (OSError, http.client.HTTPException):
            return self.answer(200, {"starting": True})
        if response.status == 200:
            job = json.loads(data).get("job", "")
            if not within_quota(self.address(), job):
                return self.answer(
                    429, {"error": "That is enough builds for today. Try again tomorrow."}
                )
        self.send_response(response.status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.seen()
        path = urllib.parse.urlparse(self.path).path
        if not path.startswith("/v1/job/"):
            return self.answer(404, {"error": "No such thing here."})
        headers = {}
        if self.headers.get("Range"):
            headers["Range"] = self.headers["Range"]
        try:
            c = connection(60)
            c.request("GET", path, headers=headers)
            response = c.getresponse()
        except (OSError, http.client.HTTPException):
            wake()
            if path.endswith("/lib"):
                return self.answer(503, {"error": "The build server is waking up."})
            return self.answer(200, {"state": "starting"})
        self.send_response(response.status)
        for name in ("Content-Type", "Content-Length", "Content-Range"):
            value = response.getheader(name)
            if value:
                self.send_header(name, value)
        self.end_headers()
        shutil.copyfileobj(response, self.wfile, 256 * 1024)


def main():
    threading.Thread(target=sleeper, daemon=True).start()
    http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
