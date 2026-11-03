#!/usr/bin/env python3
"""Send a Horizon control command to the development Android app over ADB.

The control token is read from app data and sent only over the forwarded loopback
socket. It is never printed. ADB forwards are removed when the command finishes.
"""
import argparse
import json
import socket
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', default='dev.horizon.android.dev', help='Isolated development package')
    parser.add_argument('--adb', default='adb', help='ADB executable')
    parser.add_argument('--serial', required=True, help='Explicit test device serial')
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error('Specify a command, for example state, ui, or status')
    adb = [args.adb, '-s', args.serial]
    descriptor = json.loads(subprocess.check_output([*adb, 'shell', 'run-as', args.package, 'cat', 'files/control.json']))
    port = subprocess.check_output([*adb, 'forward', 'tcp:0', f"tcp:{int(descriptor['port'])}"], text=True).strip()
    try:
        with socket.create_connection(('127.0.0.1', int(port)), timeout=60) as connection:
            stream = connection.makefile('rwb')
            stream.write(f"auth {descriptor['token']}\n".encode())
            stream.flush()
            auth = json.loads(stream.readline())
            if not auth.get('ok'):
                raise RuntimeError('Android control authentication failed')
            stream.write((' '.join(args.command) + '\n').encode())
            stream.flush()
            print(stream.readline().decode().strip())
    finally:
        subprocess.run([*adb, 'forward', '--remove', f'tcp:{port}'], check=True)


if __name__ == '__main__':
    main()
