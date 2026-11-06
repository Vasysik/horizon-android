"""Interactive, resumable console for the isolated Android benchmark matrix."""
import argparse
from collections import deque
import contextlib
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / '.local/android/benchmark-python'))
try:
    from rich.console import Console, Group
    from rich.live import Live
    from rich.panel import Panel
    from rich.progress import BarColumn, Progress, TextColumn
    from rich.table import Table
    from rich.text import Text
except ImportError:
    raise SystemExit('Install console dependencies: python -m pip install --target .local/android/benchmark-python rich')

if __package__:
    from .android_benchmark import CASES, NATURAL_CASES, DEFAULT_VERSIONS, VERSIONS, build_plan, summarize
    from .android_benchmark_device import ADB, DEVICES, PKG
else:
    from android_benchmark import CASES, NATURAL_CASES, DEFAULT_VERSIONS, VERSIONS, build_plan, summarize
    from android_benchmark_device import ADB, DEVICES, PKG


def read_json(path, default=None, *, live=False):
    try:
        return json.loads(path.read_text(encoding='utf8'))
    except (FileNotFoundError, json.JSONDecodeError):
        return default
    except PermissionError:
        if not live:
            raise
        return default


def save_json(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2), encoding='utf8')
    temporary.replace(path)


def sparkline(values, width=58):
    numbers = [v for v in list(values)[-width:] if isinstance(v, (float, int))]
    if not numbers:
        return 'Waiting for recorded samples', ''
    high = max(max(numbers), .001)
    blocks = '▁▂▃▄▅▆▇█'
    return ''.join(blocks[min(7, int(v / high * 7))] for v in numbers), f'0–{high:.1f}'


def disk_bytes(paths):
    total = 0
    for path in paths:
        if path.exists():
            for folder, _, files in os.walk(path):
                for name in files:
                    with contextlib.suppress(OSError):
                        total += (Path(folder)/name).stat().st_size
    return total


def device_lock(path):
    handle = path.open('a+b')
    if not path.stat().st_size:
        handle.write(b'0'); handle.flush()
    handle.seek(0)
    try:
        if os.name == 'nt':
            import msvcrt
            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        handle.close()
        raise SystemExit('Another benchmark console owns this device. Stop it with Q first.')
    return handle


class Dashboard:
    def __init__(self, jobs, output, device):
        self.jobs, self.output, self.device = jobs, output, device
        self.current = None
        self.phase = 'ready'
        self.phase_started = time.monotonic()
        self.started = time.monotonic()
        self.paused = False
        self.stopping = False
        self.histories = {key: deque(maxlen=90) for key in ('FPS', 'CPU ms', 'GPU ms')}
        self.sample = {}
        self.messages = deque(maxlen=5)
        self.free = shutil.disk_usage(output).free
        self.used = 0
        self.active_run = None
        self.last_sample = None
        self.finished_durations = []

    def message(self, text):
        self.messages.append(time.strftime('%H:%M:%S')+'  '+text)

    def update_run(self, run):
        self.active_run = run
        status = read_json(run/'status.json', {}, live=True)
        phase = status.get('phase', self.phase)
        if phase != self.phase:
            self.phase = phase
            self.phase_started = time.monotonic()
        row = read_json(run/'live.json', live=True)
        if row is None:
            return
        if row.get('t_ms') == self.last_sample:
            return
        self.last_sample = row.get('t_ms')
        self.sample = row
        frame = row.get('frames', {})
        values = {'FPS': frame.get('avg_fps'), 'CPU ms': (frame.get('cpu') or {}).get('avg_ms'),
                  'GPU ms': (frame.get('gpu') or {}).get('avg_ms')}
        for name, value in values.items():
            if isinstance(value, (float, int)):
                self.histories[name].append(value)

    def render(self):
        done = sum(j['status'] == 'collected' for j in self.jobs)
        failed = sum(j['status'] == 'failed' for j in self.jobs)
        state = 'STOP AFTER CURRENT' if self.stopping else 'PAUSE AFTER CURRENT' if self.paused and self.current else 'PAUSED' if self.paused else 'RUNNING'
        header = Text(f'HORIZON  /  ANDROID BENCHMARK     {self.device.upper()}     {state}', style='bold cyan')
        progress = Progress(TextColumn('[bold]{task.description}'), BarColumn(bar_width=None), TextColumn('{task.completed:.0f}/{task.total:.0f}'))
        progress.add_task('Overall trials', total=len(self.jobs), completed=done+failed)
        if self.current:
            measured = min(60, time.monotonic()-self.phase_started) if self.phase == 'measuring' else 0
            progress.add_task('Measurement seconds', total=60, completed=measured)
        trial = Table.grid(padding=(0, 2))
        trial.add_column(style='dim'); trial.add_column()
        current = self.current or {}
        trial.add_row('Build', str(current.get('version', '—')))
        trial.add_row('Renderer', str(current.get('renderer', '—')))
        trial.add_row('Scene', str(current.get('scenario', '—')))
        trial.add_row('Distance / LOD', f"{current.get('distance', '—')} / {current.get('lod', '—')} chunks")
        trial.add_row('Repeat', str(current.get('repeat', -1)+1) if current else '—')
        trial.add_row('Phase', self.phase)
        trial.add_row('Phase elapsed', f'{time.monotonic()-self.phase_started:.0f} s')
        elapsed = time.monotonic()-self.started
        eta = 'Estimating'
        if self.finished_durations:
            seconds = sum(self.finished_durations)/len(self.finished_durations)*(len(self.jobs)-done-failed)
            eta = f'{seconds/3600:.1f} h remaining (approx.)'
        context = self.sample.get('context', {})
        renderer = context.get('renderer', {})
        resources = Table.grid(padding=(0, 2))
        resources.add_row('Elapsed / ETA', f'{elapsed/60:.1f} min / {eta}')
        resources.add_row('Disk free / session', f'{self.free/2**30:.2f} / {self.used/2**30:.2f} GiB')
        resources.add_row('Collected / failed', f'{done} / {failed}')
        resources.add_row('Draw calls', str(renderer.get('draw_calls', '—')))
        resources.add_row('GPU allocated', f"{renderer.get('gpu_allocated_bytes', 0)/2**20:.0f} MiB")
        resources.add_row('Thermal status', str(context.get('android_power', {}).get('thermal_status', '—')))
        columns = Table.grid(expand=True); columns.add_column(ratio=1); columns.add_column(ratio=1)
        columns.add_row(Panel(trial, title='Current trial', border_style='cyan'), Panel(resources, title='Resources', border_style='blue'))
        charts = Table.grid(padding=(0, 2)); charts.add_column(width=9); charts.add_column(); charts.add_column()
        for name, color in (('FPS', 'green'), ('CPU ms', 'yellow'), ('GPU ms', 'magenta')):
            line, scale = sparkline(self.histories[name])
            values = self.histories[name]
            charts.add_row(Text(name, style=color), Text(line, style=color), Text(f'{values[-1]:.2f}  [{scale}]' if values else '—'))
        graphs = Panel(charts, title='Recorder windows · CPU/GPU are timings, not utilization', border_style='green')
        logs = Panel(Text('\n'.join(self.messages) or 'Preparing...'), title='Events', border_style='dim')
        review = ''
        if self.phase == 'awaiting-review' and self.active_run:
            review = '\nReview required: '+str(self.active_run/'scene-preview.png')
        footer = Text('P  pause/resume at trial boundary     Q / Ctrl+C  stop after current trial'+review, style='bold yellow')
        return Group(Panel(header, border_style='cyan'), progress, columns, graphs, logs, footer)


def pressed_key():
    if os.name == 'nt':
        import msvcrt
        if msvcrt.kbhit():
            return msvcrt.getwch().lower()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixtures', type=Path, default=ROOT/'.local/android/regression-073-077')
    parser.add_argument('--device', choices=DEVICES, default='tablet')
    parser.add_argument('--session', default='tablet-console-20261104')
    parser.add_argument('--versions', nargs='+', choices=VERSIONS, default=list(DEFAULT_VERSIONS))
    parser.add_argument('--renderers', nargs='+', choices=('compatibility', 'batching', 'opengl'), default=['compatibility'])
    ids = {f'{s}-{d}-{l}': (s, d, l) for s, d, l in CASES}
    parser.add_argument('--cases', nargs='+', choices=ids,
                        default=[f'{s}-{d}-{l}' for s,d,l in NATURAL_CASES])
    parser.add_argument('--repeats', type=int, default=2)
    parser.add_argument('--unattended', action='store_true', help='Do not wait for manual/agent scene review')
    parser.add_argument('--retry-failed', action='store_true')
    parser.add_argument('--continue-on-error', action='store_true', help='Record failed trials and keep testing other configurations')
    parser.add_argument('--demo', action='store_true', help='Show simulated UI only; never contact a device')
    args = parser.parse_args()
    if Path(args.session).name != args.session or args.session in ('.', '..') or args.repeats < 1:
        parser.error('Invalid session or repeat count')
    fixtures = args.fixtures.resolve()
    output = fixtures/args.session
    output.mkdir(parents=True, exist_ok=True)
    proposed = build_plan([ids[x] for x in args.cases], tuple(args.versions), args.repeats, args.renderers)
    journal = output/'console-jobs.json'
    existing = read_json(journal)
    if existing and [j['key'] for j in existing] != [j['key'] for j in proposed]:
        parser.error('Session has another plan. Use a new --session name.')
    jobs = existing or proposed
    for job in jobs:
        if job['status'] == 'running' or (args.retry_failed and job['status'] == 'failed'):
            job['status'] = 'pending'
    if any(j['status'] == 'failed' for j in jobs) and not args.retry_failed and not args.continue_on_error:
        parser.error('Inspect failed trials first, then resume with --retry-failed')
    board = Dashboard(jobs, output, args.device)
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf8')
    console = Console(legacy_windows=False)
    if args.demo:
        board.phase = 'DEMO · not measurements'
        board.message('Demonstration only. No Android commands sent.')
        for i in range(60):
            for index, values in enumerate(board.histories.values()):
                values.append(35+index*10+(i%11))
        console.print(board.render())
        return
    if not sys.stdin.isatty():
        parser.error('Run in an interactive PowerShell terminal for P/Q controls.')
    names = [f'Horizon-{v}-benchmark.apk' if v in ('0.7.3','0.7.7') else f'cpu-{v}.apk' for v in args.versions]
    hashes = {name: hashlib.sha256((fixtures/name).read_bytes()).hexdigest() for name in names}
    old_hashes = read_json(output/'apks.json')
    if old_hashes and old_hashes != hashes:
        parser.error('APK identity changed. Use a new session instead of mixing builds.')
    save_json(output/'apks.json', hashes)
    serial = DEVICES[args.device][0]
    env = dict(os.environ, HORIZON_BENCH_FIXTURES=str(fixtures), HORIZON_BENCH_SESSION=output.name,
               HORIZON_BENCH_MANIFEST=str(output/'apks.json'), PYTHONIOENCODING='utf-8')
    process = None
    log = None
    current_started = 0
    last_disk = 0
    waiting_device = True
    lock = device_lock(fixtures/(args.device+'.console.lock'))
    old_signal = signal.getsignal(signal.SIGINT)
    signal.signal(signal.SIGINT, lambda *_: setattr(board, 'stopping', True))
    try:
        with Live(board.render(), console=console, refresh_per_second=4, screen=True) as live:
            while True:
                key = pressed_key()
                if key == 'p':
                    board.paused = not board.paused
                    board.message('Pause requested; current trial will finish.' if board.paused else 'Resuming queue.')
                if key == 'q':
                    board.stopping = True
                    board.message('Stop requested; collecting current trial before exit.')
                now = time.monotonic()
                if now-last_disk > 10:
                    board.free = shutil.disk_usage(output).free
                    board.used = disk_bytes([output]+[Path(j['run']).parent for j in jobs if j.get('run')])
                    last_disk = now
                if process is not None:
                    board.update_run(Path(board.current['run']))
                    if board.stopping and board.phase == 'awaiting-review':
                        (Path(board.current['run'])/'cancel-review.flag').touch()
                    code = process.poll()
                    if code is not None:
                        log.close(); log = None
                        job = board.current
                        cancelled = board.phase == 'cancelled'
                        job.update(status='pending' if cancelled else 'collected' if code == 0 else 'failed', exit_code=code, elapsed_seconds=now-current_started)
                        board.finished_durations.append(now-current_started)
                        save_json(journal, jobs)
                        board.message(f"{job['scenario']} / {job['version']}: {job['status']}")
                        if code and not args.continue_on_error:
                            board.stopping = True
                            board.message('Stopped on error. Inspect '+job['log'])
                        process = None; board.current = None
                        summarize(fixtures, output/'results.json')
                if process is None:
                    if board.stopping:
                        break
                    pending = next((j for j in jobs if j['status'] == 'pending'), None)
                    if pending is None:
                        board.message('Queue complete. Scene review remains separate from successful execution.')
                        break
                    if not board.paused:
                        if waiting_device:
                            probe = subprocess.run([ADB, '-s', serial, 'shell', 'pidof', PKG], capture_output=True, timeout=15)
                            if probe.stdout.strip():
                                board.phase = 'waiting-for-existing-test'
                                board.message('Existing test app is running. Waiting; no second driver launched.')
                                live.update(board.render()); time.sleep(3)
                                continue
                            waiting_device = False
                        if board.free < 2*2**30:
                            board.paused = True
                            board.message('Less than 2 GiB free. Queue paused before starting another trial.')
                            continue
                        job = pending
                        trial_id = args.device+'-console-'+uuid.uuid4().hex[:10]
                        run = fixtures/'cpu-experiments'/trial_id/('1-'+job['version'])
                        job.update(status='running', run=str(run), log=str(output/(trial_id+'.log')))
                        save_json(journal, jobs)
                        command = [sys.executable, str(ROOT/'tools/android_benchmark_device.py'), args.device,
                                   job['scenario'], '--distance', str(job['distance']), '--lod-distance', str(job['lod']),
                                   '--renderer', job['renderer'],
                                   '--versions', job['version'], '--trial-id', trial_id, '--live']
                        if not args.unattended:
                            command.append('--review-gate')
                        if job['scenario'].endswith('-flight'):
                            command.append('--prepare-route')
                        log = open(job['log'], 'w', encoding='utf8')
                        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
                        board.current = job; board.phase = 'starting'; board.phase_started = now
                        board.last_sample = None; board.sample = {}; board.active_run = run
                        for history in board.histories.values():
                            history.clear()
                        current_started = now
                        board.message('Started '+job['scenario']+' / '+job['version'])
                live.update(board.render())
                time.sleep(.2)
    finally:
        signal.signal(signal.SIGINT, old_signal)
        if process is not None:
            console.print('Waiting for current trial to restore device settings...')
            if board.active_run and (board.active_run/'scene-preview.png').exists():
                (board.active_run/'cancel-review.flag').touch()
            process.wait()
        if log:
            log.close()
        lock.close()
    console.print(f'Collected {sum(j["status"] == "collected" for j in jobs)}/{len(jobs)}. Session: {output}')


if __name__ == '__main__':
    main()
