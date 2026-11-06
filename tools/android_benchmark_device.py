"""Device driver for isolated benchmarks with disposable copies of controlled saves."""
import argparse
import os
import hashlib
import json
import pathlib
import re
import socket
import math
import subprocess
import time
import uuid

PKG = "dev.horizon.android.regression076"
ADB = "C:/Users/vasia/AppData/Local/Android/Sdk/platform-tools/adb.exe"
DEVICES = {
    "tablet": ("192.168.88.234:5555", "751097", "GPUSeed4240118709374351666", (380, 1430), (1280, 740), (1050, 370), (770, 1350), (1280, 1086)),
    "poco": ("75b95755", "751085", "PocoSeed4240118709374351666", (380, 1100), (1386, 610), (1100, 310), (1000, 1080), (1386, 870)),
}
BIOMES = {"ocean-flight":"ocean", "forest-flight":"forest", "jungle-flight":"jungle",
          "desert-flight":"desert", "snow-flight":"snowy_plains"}
NATURAL = (*BIOMES, "nether-flight", "end-flight")
SCENARIOS = (*NATURAL, "nether", "end", "water", "foliage", "sand", "jungle-lab", "terrain", "entities-control", "entities-static", "entities-tick-control", "entities-ticking", "stream-warm", "stream-cold", "lod-off", "lod-on", "flight-cold", "flight-warm", "camera-orbit")
SETUP = ("/gamemode creative", "/time set 6000", "/weather clear", "/gamerule spawn_mobs false")
CAMERA = "/tp @s -24.5 102 -47.5 0 0"
# Spread 64 zombies across the view. NoAI isolates rendering from mob AI.
ZOMBIES = tuple(
    '/summon minecraft:zombie %d 104 %d {NoAI:1b,Silent:1b,PersistenceRequired:1b,Tags:["cpu_probe"]}' % (x,z)
    for x in range(-36, -12, 3) for z in range(-37, -13, 3)
)
COLD = ("/tp @s 4096 150 4096 0 0", "/tp @s 4352 150 4096 0 0", "/tp @s 4608 150 4096 0 0")


# Controlled material workloads, not claims about natural desert/jungle biomes.
# Every fill stays below vanilla's command volume limit. Worlds are disposable copies.
LAB = {
    "terrain": ["/tp @s -24.5 102 -47.5 0 0"],
    "sand": ["/fill -56 100 -16 7 100 47 minecraft:sandstone",
             "/fill -56 101 -16 7 101 47 minecraft:sand",
             "/tp @s -24.5 104 -22.5 0 0"],
    "water": ["/fill -56 100 -16 7 100 47 minecraft:stone",
              "/fill -56 101 -16 7 104 47 minecraft:water",
              "/tp @s -24.5 106 -22.5 0 8"],
    "foliage": ["/fill -56 100 -16 7 100 47 minecraft:stone",
                "/fill -56 104 -16 7 108 47 minecraft:oak_leaves[persistent=true]",
                "/tp @s -24.5 110 -22.5 0 8"],
    "jungle-lab": ["/fill -56 100 -16 7 100 47 minecraft:stone",
                  "/fill -56 104 -16 7 108 47 minecraft:jungle_leaves[persistent=true]",
                  *[f"/fill {x} 101 {z} {x} 112 {z} minecraft:jungle_log"
                    for x in range(-48,1,16) for z in range(0,33,16)],
                  "/tp @s -24.5 103 -22.5 0 0"],
}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("device", choices=DEVICES)
    parser.add_argument("scenario", choices=SCENARIOS)
    parser.add_argument("--versions", nargs="+", choices=["0.7.3","0.7.7","0.7.8","0.7.8-f42ef42"], default=["0.7.3","0.7.7","0.7.8"])
    parser.add_argument("--terrain", choices=("single","split"))
    parser.add_argument("--render-scale", type=int, choices=(50, 75, 100), default=100)
    parser.add_argument("--distance", type=int, choices=(12, 16, 24))
    parser.add_argument("--renderer", choices=("compatibility", "batching", "opengl"), default="compatibility")
    parser.add_argument("--lod-distance", type=int, choices=(0, 64, 128))
    parser.add_argument("--describe", action="store_true")
    parser.add_argument("--external-profiler", choices=("none", "simpleperf", "perfetto", "gpu"), default="none")
    parser.add_argument("--trial-id", help="Unique private output directory name")
    parser.add_argument("--live", action="store_true", help="Publish sampled recorder data for the console")
    parser.add_argument("--review-gate", action="store_true", help="Wait for scene validation before measuring")
    parser.add_argument("--prepare-route", action="store_true", help="Discover an unapproved real-biome route")
    parser.add_argument("--preview-only", action="store_true", help="Save scene checks/screenshots without a benchmark result")
    args = parser.parse_args()
    fixture_directory = os.environ.get("HORIZON_BENCH_FIXTURES")
    if not fixture_directory:
        parser.error("Set HORIZON_BENCH_FIXTURES to the private fixture directory")
    P = pathlib.Path(fixture_directory).resolve()
    serial, source_profile, source_world, play, single, row, open_world, leave = DEVICES[args.device]
    scenario = args.scenario
    distance = 16 if scenario.startswith("entities") else 24
    lod = 64 if scenario == "lod-on" else 0
    if args.distance is not None:
        distance = args.distance
    if args.lod_distance is not None:
        lod = args.lod_distance
    if args.describe:
        print(json.dumps(dict(device=serial, scenario=scenario, versions=args.versions, distance=distance, lod=lod, fixture=source_world, mobs=len(ZOMBIES) if scenario in ("entities-static","entities-tick-control","entities-ticking") else 0), indent=2))
        return

    def adb(*cmd, data=None):
        return subprocess.run([ADB,"-s",serial,*cmd],input=data,stdout=subprocess.PIPE,check=True,timeout=90).stdout
    def sh(*cmd): return adb("shell",*cmd)
    def app(*cmd): return sh("run-as",PKG,*cmd)
    def put(path, data): adb("shell","run-as",PKG,"tee",path,data=data)
    def tap(pos): sh("input","tap",*map(str,pos))
    def key(code): sh("input","keyevent",str(code))
    def control_reply(stream, stage):
        line = stream.readline(1024 * 1024)
        if not line:
            raise ConnectionError(f"Game control closed during {stage}")
        return json.loads(line)

    def ctl(command):
        # Retry only connection/authentication. A dispatched command may already
        # have executed even if its reply was lost.
        for attempt in range(3):
            endpoint = json.loads(app("cat", "files/control.json"))
            port = adb("forward", "tcp:0", "tcp:"+str(int(endpoint["port"]))).decode().strip()
            dispatched = False
            try:
                with socket.create_connection(("127.0.0.1", int(port)), timeout=3) as connection:
                    with connection.makefile("rwb") as stream:
                        stream.write(("auth "+endpoint["token"]+"\n").encode())
                        stream.flush()
                        if not control_reply(stream, "authentication").get("ok"):
                            raise ConnectionError("Game control rejected authentication")
                        connection.settimeout(120)
                        dispatched = True
                        stream.write((command+"\n").encode())
                        stream.flush()
                        response = control_reply(stream, command.split()[0])
                        if response.get("error"):
                            raise RuntimeError(response["error"])
                        return response
            except (ConnectionError, TimeoutError):
                if dispatched or attempt == 2:
                    raise
                time.sleep(.4)
            finally:
                adb("forward", "--remove", "tcp:"+port)

    def chat(command):
        return ctl("chat "+command)

    def motion(duration, orbit=False, tour=False, on_sample=None):
        """Real held movement, or a timed camera sweep, through existing game input."""
        endpoint = json.loads(app("cat", "files/control.json"))
        port = adb("forward", "tcp:0", "tcp:" + str(int(endpoint["port"]))).decode().strip()
        try:
            with socket.create_connection(("127.0.0.1", int(port)), timeout=5) as conn:
                conn.settimeout(10)
                with conn.makefile("rwb") as stream:
                    def send(command):
                        stream.write((command + "\n").encode())
                        stream.flush()
                        line = stream.readline(1024 * 1024)
                        if not line:
                            raise RuntimeError("Game control connection closed during motion")
                        result = json.loads(line)
                        if result.get("error"):
                            raise RuntimeError("Game rejected motion command: " + result["error"])
                        return result
                    send("auth " + endpoint["token"])
                    initial = send("status")
                    started = time.monotonic()
                    updates = 0
                    max_update_gap = 0.0
                    previous = started
                    next_sample = started
                    held = False
                    try:
                        if not orbit and not tour:
                            send("key w down")
                            held = True
                        while (elapsed := time.monotonic() - started) < duration:
                            now = time.monotonic()
                            max_update_gap = max(max_update_gap, now - previous)
                            previous = now
                            if on_sample and now >= next_sample:
                                on_sample(); next_sample = now+2
                            if orbit:
                                send(f"look {elapsed * 360 / duration:.5f} 0")
                                updates += 1
                            elif tour:
                                # Alternate actual movement and hovering sweeps.
                                # Pitch sweeps include both ground and sky views.
                                segment = int(elapsed // 10)
                                moving = segment % 2 == 0
                                if moving and not held:
                                    send("look 0 0"); send("key w down"); held = True
                                elif not moving:
                                    if held:
                                        send("key w up"); held = False
                                    t = elapsed % 10
                                    send(f"look {t*36:.4f} {65*math.sin(t*math.tau/10):.4f}")
                                    updates += 1
                            time.sleep(min(.05, max(0, duration - elapsed)))
                    finally:
                        if held:
                            send("key w up")
                    return {"initial": initial, "final": send("status"),
                            "duration_seconds": time.monotonic() - started,
                            "camera_updates": updates, "max_update_gap_seconds": max_update_gap,
                            "input": "alternating held W and hovering yaw/pitch sweeps" if tour else "absolute camera yaw, 20 Hz target" if orbit else "held W"}
        finally:
            adb("forward", "--remove", "tcp:" + port)

    trial_id = args.trial_id or args.device+"-"+scenario+"-"+uuid.uuid4().hex[:8]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", trial_id) or trial_id in (".", ".."):
        parser.error("Invalid trial ID")
    out = P/"cpu-experiments"/trial_id
    out.mkdir(parents=True)
    launcher_path, launch_path = "files/android-launcher.json", "files/android-launch.json"
    originals = {name: app("cat",name) for name in (launcher_path,launch_path)}
    completed_homes = []
    source = "files/profiles/profile-"+source_profile
    options = app("cat",source+"/options.txt")
    for k,v in dict(renderDistance=distance, simulationDistance=12, textureFiltering=0, maxFps=120, inactivityFpsLimit='"minimized"', onboardAccessibility="false").items():
        options = re.sub(rb"(?m)^"+k.encode()+rb":[^\r\n]*\r?\n?", b"", options)
        options += f"\n{k}:{v}\n".encode()
    try:
        for i,version in enumerate(args.versions,1):
            run = out/f"{i}-{version}"
            run.mkdir()
            events = []
            def publish(name, value):
                temporary = run/(name+".tmp")
                temporary.write_text(json.dumps(value, ensure_ascii=False), encoding="utf8")
                for attempt in range(5):
                    try:
                        temporary.replace(run/name)
                        return
                    except PermissionError:
                        if attempt < 4:
                            time.sleep(.02)
                        elif name not in ("live.json", "status.json"):
                            raise
            def phase(name):
                publish("status.json", {"phase":name,"host_monotonic_ns":time.monotonic_ns()})
            def live_sample():
                if not args.live:
                    return
                samples = []
                for filename in set(app("ls", "files/performance").decode().splitlines()) - before:
                    for line in app("tail", "-c", "65536", "files/performance/"+filename).splitlines():
                        try:
                            row = json.loads(line)
                        except ValueError:
                            continue
                        if row.get("type") == "sample":
                            samples.append(row)
                if samples:
                    publish("live.json", max(samples, key=lambda row: row.get("t_ms", 0)))
            def wait_measured(seconds):
                deadline = time.monotonic()+seconds
                while time.monotonic() < deadline:
                    live_sample()
                    time.sleep(min(2, max(0, deadline-time.monotonic())))
            def report_clock():
                clocks = []
                for filename in set(app("ls", "files/performance").decode().splitlines()) - before:
                    for line in app("cat", "files/performance/" + filename).splitlines():
                        try:
                            sample = json.loads(line)
                        except ValueError:
                            continue
                        if sample.get("type") == "sample" and isinstance(sample.get("t_ms"), (int, float)):
                            clocks.append(sample["t_ms"])
                return max(clocks, default=None)
            def event(name):
                events.append({"name":name,"host_monotonic_ns":time.monotonic_ns(),
                               "report_t_ms":report_clock()})
                (run/"events.json").write_text(json.dumps(events, indent=2), encoding="utf8")
            def shot(name): (run/(name+".png")).write_bytes(adb("exec-out","screencap","-p"))
            sh("am","force-stop",PKG)
            app("rm","-f","files/control.json")
            phase("cooling")
            cooling_started = time.monotonic()
            while True:
                thermal_report = sh("dumpsys", "thermalservice")
                status = re.search(rb"Thermal Status:\s*(\d+)", thermal_report)
                if status is None or int(status[1]) <= 1:
                    break
                if time.monotonic() - cooling_started > 300:
                    raise RuntimeError("Tablet remains thermally constrained; aborting rather than comparing hot runs")
                time.sleep(10)
            thermal_wait_seconds = time.monotonic() - cooling_started
            apk = P / (f"Horizon-{version}-benchmark.apk" if version in ("0.7.3", "0.7.7") else f"cpu-{version}.apk")
            apk_hash = hashlib.sha256(apk.read_bytes()).hexdigest()
            manifest_path = os.environ.get("HORIZON_BENCH_MANIFEST")
            if manifest_path and json.loads(pathlib.Path(manifest_path).read_text())[apk.name] != apk_hash:
                raise RuntimeError("APK changed during benchmark; refusing to mix builds")
            phase("installing")
            print(version,adb("install","-r",str(apk)).decode().strip(),flush=True)
            if hashlib.sha256(apk.read_bytes()).hexdigest() != apk_hash:
                raise RuntimeError("APK changed during installation")
            profile=2_000_000_000+int(uuid.uuid4().hex[:7],16)%100_000_000
            home=f"files/profiles/profile-{profile}"
            app("mkdir","-p",home+"/saves")
            app("cp","-R",source+"/saves/"+source_world,home+"/saves/")
            if scenario.startswith("entities") or scenario in LAB:
                base = home+"/saves/"+source_world+"/datapacks/cpu_probe"
                app("mkdir","-p",base+"/data/cpu_probe/function")
                put(base+"/pack.mcmeta",json.dumps({"pack":{"description":"CPU test fixture","min_format":[121,0],"max_format":[121,0]}}).encode())
                commands = [] if scenario == "entities-control" else [command.replace("NoAI:1b,","") if scenario == "entities-ticking" else command for command in ZOMBIES]
                if scenario in ("entities-tick-control","entities-ticking"):
                    commands = ["/fill -37 103 -38 -14 103 -15 minecraft:stone"] + [command.replace("minecraft:zombie","minecraft:husk") for command in commands]
                if scenario in LAB:
                    commands = LAB[scenario]
                put(base+"/data/cpu_probe/function/spawn.mcfunction",("\n".join(command.lstrip("/") for command in commands)+"\n").encode())
            put(home+"/options.txt",options)
            put(home+"/horizon-graphics.txt",f"lodDistance:{lod}\n".encode())
            cfg=json.loads(originals[launcher_path])
            renderer_mode = {"compatibility":"vulkan-compatibility", "batching":"vulkan", "opengl":"open-gl"}[args.renderer]
            cfg.update(profile=profile,record_performance=True,gpu_profile=True,performance_hints=False,
                       display_hz=0,worker_limit=0,cache_budget_mib=0,
                       render_scale=args.render_scale,renderer=renderer_mode)
            put(launcher_path,json.dumps(cfg).encode())
            put(launch_path,b"[]")
            before=set(app("ls","files/performance").decode().splitlines())
            (run/"controlled-options.txt").write_bytes(options)
            (run/"device-model.txt").write_bytes(sh("getprop", "ro.product.model"))
            (run/"memory-before.txt").write_bytes(sh("cat", "/proc/meminfo"))
            (run/"thermal-before.txt").write_bytes(sh("dumpsys","thermalservice"))
            key(224)
            sh("wm","dismiss-keyguard")
            sh("am","start","-W","-n",PKG+"/com.google.androidgamesdk.GameActivity")
            phase("launching")
            event("launcher-started")
            play_button = (380, 1150) if args.device == "poco" and version in ("0.7.3", "0.7.7") else play
            time.sleep(5);tap(play_button);time.sleep(10)
            if args.terrain:
                endpoint=json.loads(app("cat","files/control.json"))
                response=adb("shell","run-as",PKG,"toybox","nc","-w","12","127.0.0.1",str(endpoint["port"]),data=("auth "+endpoint["token"]+"\nterrain-passes "+args.terrain+"\n").encode())
                replies=[]
                for line in response.splitlines():
                    try: replies.append(json.loads(line))
                    except ValueError: pass
                if not replies or not replies[-1].get("ok"):
                    raise RuntimeError("terrain command failed")
                (run/"terrain-mode.txt").write_text(args.terrain)
            tap(single);time.sleep(3);tap(row);tap(open_world)
            phase("loading-world")
            deadline = time.monotonic() + 45
            while True:
                active = False
                for name in set(app("ls","files/performance").decode().splitlines())-before:
                    for line in app("tail","-n","20","files/performance/"+name).splitlines():
                        try: sample = json.loads(line)
                        except ValueError: continue
                        active |= bool(sample.get("context", {}).get("in_world"))
                if active:
                    break
                if time.monotonic() >= deadline:
                    shot("world-not-open")
                    raise RuntimeError("No in-world report. Check the launcher and world-selection screenshots.")
                time.sleep(1)
            if sh("run-as", PKG, "test", "-s", "files/control.json", "&&", "echo", "ready", "||", "true").strip() != b"ready":
                raise RuntimeError(
                    f"{version} did not start benchmark control. Use an isolated APK "
                    "built with development control enabled for this version."
                )
            event("world-entered")
            phase("setting-up")
            for command in (*SETUP, CAMERA): chat(command)
            if scenario in NATURAL:
                chat("/gamemode spectator")
                routes_path = P/"natural-routes.json"
                routes = json.loads(routes_path.read_text()) if routes_path.exists() else {}
                route_key = args.device+":"+scenario
                route = routes.get(route_key)
                if route is None:
                    if not args.prepare_route:
                        raise RuntimeError("Route not prepared: "+route_key)
                    if scenario in BIOMES:
                        target = "minecraft:"+BIOMES[scenario]
                        chat("/locate biome "+target)
                        coordinates = None
                        deadline = time.monotonic()+120
                        while time.monotonic()<deadline:
                            text = json.dumps(ctl("chat-log 8"), ensure_ascii=False)
                            matches = re.findall(r"\[(-?\d+),\s*(-?\d+|~),\s*(-?\d+)\]", text)
                            if matches:
                                coordinates = matches[-1]; break
                            time.sleep(1)
                        if coordinates is None:
                            (run/"locate-response.json").write_text(text)
                            raise RuntimeError("No biome coordinates in locate response")
                        x, z = int(coordinates[0]), int(coordinates[2])
                        y = 95 if scenario == "ocean-flight" else 160
                        route = {"dimension":"minecraft:overworld", "position":[x+.5,y,z+.5], "yaw":0,"pitch":25,"biome":target,"approved":False}
                    elif scenario == "end-flight":
                        route = {"dimension":"minecraft:the_end", "position":[0.5,100,-60.5],"yaw":0,"pitch":25,"approved":False}
                    else:
                        route = {"dimension":"minecraft:the_nether", "position":[0.5,64,0.5],"yaw":0,"pitch":0,"approved":False}
                    routes[route_key] = route
                    routes_path.write_text(json.dumps(routes,indent=2))
                if not route.get("approved") and not args.prepare_route:
                    raise RuntimeError("Route is awaiting visual/path validation: "+route_key)
                x,y,z = route["position"]
                chat(f'/execute in {route["dimension"]} run tp @s {x} {y} {z} {route["yaw"]} {route["pitch"]}')
                deadline = time.monotonic() + 20
                while True:
                    state = ctl("status")
                    position = (state.get("player") or {}).get("position")
                    if (state.get("dimension") == route["dimension"] and position
                            and math.dist(position, (x, y, z)) < 2):
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Camera did not reach the requested route start")
                    time.sleep(.5)
                eye = [math.floor(x),math.floor(y+1.62),math.floor(z)]
                block = ctl("block "+" ".join(map(str,eye)))
                air = ("minecraft:air","minecraft:cave_air","minecraft:void_air")
                if block.get("block") not in air and args.prepare_route:
                    candidates = ((x+dx, yy, z+dz) for yy in (64,48,80,32,96)
                                  for dx,dz in ((0,0),(16,0),(-16,0),(0,16),(0,-16),(32,32))) if scenario == "nether-flight" else ((x,y+dy,z) for dy in (20,40,60,80,100))
                    found = None
                    for cx,cy,cz in candidates:
                        check = ctl(f"block {math.floor(cx)} {math.floor(cy+1.62)} {math.floor(cz)}")
                        ahead = ctl(f"block {math.floor(cx)} {math.floor(cy+1.62)} {math.floor(cz+8)}")
                        if check.get("block") in air and ahead.get("block") in air:
                            found = (cx,cy,cz); break
                    if found:
                        x,y,z = found
                        route["position"] = list(found)
                        routes[route_key] = route
                        routes_path.write_text(json.dumps(routes,indent=2))
                        chat(f'/tp @s {x} {y} {z} {route["yaw"]} {route["pitch"]}')
                        time.sleep(2)
                        state = ctl("status")
                        block = ctl(f"block {math.floor(x)} {math.floor(y+1.62)} {math.floor(z)}")
                (run/"scene-state.json").write_text(json.dumps({"route":route,"state":state,"eye_block":block},indent=2))
                if state.get("dimension") != route["dimension"]:
                    raise RuntimeError("Dimension switch did not complete")
                if block.get("block") not in ("minecraft:air","minecraft:cave_air","minecraft:void_air"):
                    shot("camera-blocked")
                    raise RuntimeError("Camera is inside "+str(block.get("block"))+"; choose an open route")
            if scenario in ("nether", "end"):
                dimension = "the_nether" if scenario == "nether" else "the_end"
                chat(f"/execute in minecraft:{dimension} run tp @s 0 90 0 0 0")
                time.sleep(30)
            if scenario.startswith("entities"):
                # Freeze simulation in both runs. Compare a fixed view with/without
                # visible entities, independent of server AI.
                chat("/tick unfreeze")
                chat("/kill @e[type=!minecraft:player]")
                time.sleep(4)
                chat("/tick freeze")
                if scenario != "entities-control":
                    chat("/function cpu_probe:spawn")
                if scenario in ("entities-tick-control","entities-ticking"): chat("/tick unfreeze")
            if scenario == "stream-warm":
                for command in COLD:
                    chat(command);time.sleep(30)
                chat(CAMERA)
            if scenario in ("flight-cold", "flight-warm"):
                # Spectator flight exercises continuous streaming without trees
                # or terrain collisions changing the route between versions.
                chat("/gamemode spectator")
                chat("/tp @s 4096 150 4096 0 0")
                if scenario == "flight-warm":
                    (run/"warm-route.json").write_text(json.dumps(motion(60), indent=2))
                    chat("/tp @s 4096 150 4096 0 0")
            if scenario in LAB:
                chat("/tick unfreeze")
                chat("/function cpu_probe:spawn")
                time.sleep(8)
                chat("/tick freeze")
            phase("warming-up")
            time.sleep(5 if args.preview_only else 30)
            effective = ctl("status")
            backend = str((effective.get("renderer") or {}).get("backend", "")).lower()
            expected_backends = ("gl", "opengl") if args.renderer == "opengl" else ("vulkan",)
            if backend not in expected_backends:
                publish("wrong-backend.json", effective)
                raise RuntimeError("Requested "+args.renderer+" but effective backend is "+backend)
            publish("effective-state.json", effective)
            (run/"run.json").write_text(json.dumps({
                "device":serial, "scenario":scenario, "version":version,
                "session":os.environ.get("HORIZON_BENCH_SESSION"),
                "apk_sha256":apk_hash,
                "distance":distance, "lod":lod, "render_scale":args.render_scale,
                "filtering":"none", "renderer":renderer_mode,
                "simulation_distance":12, "frame_limit":120,
                "thermal_wait_seconds":thermal_wait_seconds,
                "instrumentation":{"performance_recording":True,"gpu_profile":True,"external_profiler":args.external_profiler},
                "dashboard_live_sampling":args.live,
                "scene_kind":"synthetic" if scenario in LAB else "copied-natural-world",
            },indent=2),encoding="utf8")
            if scenario in NATURAL:
                publish("validation.json", {"valid":True,"method":"automated dimension and camera-air checks", "visual_review":False})
            if args.review_gate:
                (run/"validation.json").unlink(missing_ok=True)
                shot("scene-preview")
                phase("awaiting-review")
                while True:
                    if (run/"cancel-review.flag").exists():
                        phase("cancelled")
                        return
                    validation_path = run/"validation.json"
                    try:
                        review = json.loads(validation_path.read_text(encoding="utf8"))
                    except (FileNotFoundError, json.JSONDecodeError):
                        review = None
                    if review is not None:
                        if not review.get("valid"):
                            raise RuntimeError("Scene rejected: "+review.get("reason", "invalid fixture"))
                        break
                    time.sleep(1)
            if args.preview_only:
                shot("scene-preview")
                publish("preview-state.json", ctl("status"))
                phase("preview-complete")
                return
            phase("measuring")
            event("measure-start")
            shot("measure-start")
            if scenario in NATURAL:
                (run/"motion.json").write_text(json.dumps(motion(60,tour=True,on_sample=live_sample),indent=2))
            elif scenario in ("flight-cold", "flight-warm", "camera-orbit"):
                (run/"motion.json").write_text(json.dumps(motion(60, orbit=scenario == "camera-orbit"), indent=2))
            elif scenario in ("stream-warm","stream-cold"):
                for command in COLD:
                    chat(command);event(command);time.sleep(12)
            else: wait_measured(60)
            event("measure-end");shot("measure-end")
            phase("collecting")
            key(111);time.sleep(2);shot("pause")
            tap(leave);event("exit-request")
            time.sleep(2);shot("exit-2s")
            time.sleep(6);shot("exit-8s")
            for name in set(app("ls","files/performance").decode().splitlines())-before:
                (run/pathlib.Path(name).name).write_bytes(app("cat","files/performance/"+name))
            (run/"events.json").write_text(json.dumps(events,indent=2),encoding="utf-8")
            (run/"thermal-after.txt").write_bytes(sh("dumpsys","thermalservice"))
            (run/"exit-info.txt").write_bytes(sh("dumpsys","activity","exit-info",PKG))
            completed_homes.append(home)
            phase("complete")
            print("captured",run,flush=True)
    except Exception as error:
        # Preserve evidence even when startup, OpenGL or fixture preparation fails.
        if "run" in locals():
            (run/"error.json").write_text(json.dumps({"error":str(error),"version":version,
                "scenario":scenario,"renderer":args.renderer,"apk_sha256":locals().get("apk_hash")},indent=2))
            for filename, command in (("failure-exit-info.txt",("dumpsys","activity","exit-info",PKG)),
                                      ("failure-thermal.txt",("dumpsys","thermalservice"))):
                try: (run/filename).write_bytes(sh(*command))
                except Exception: pass
            try:
                (run/"failure-logcat.txt").write_bytes(adb("logcat","-d","-t","400","-v","threadtime","-s","AndroidRuntime","libc","DEBUG","Horizon"))
            except Exception: pass
        raise
    finally:
        sh("am","force-stop",PKG)
        for path,data in originals.items(): put(path,data)
        # Only profiles allocated by this invocation, in the isolated package.
        # Keep a failed trial's profile for inspection; never touch source profiles.
        for home in completed_homes:
            if not re.fullmatch(r"files/profiles/profile-2[0-9]{9}", home):
                raise RuntimeError("Refusing to remove an unexpected test profile path")
            app("rm", "-r", "--", home)

if __name__ == "__main__": main()








