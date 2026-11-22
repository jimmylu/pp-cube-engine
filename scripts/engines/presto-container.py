#!/usr/bin/env python3
"""Provision one disposable, pinned Presto engine and execute the smoke golden."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
# Presto 0.293 linux/amd64 manifest, not a mutable tag.
IMAGE = 'prestodb/presto@sha256:6e7f7b4470522f5599a3b42e85036a526256d51b532fe8e6098d88ff8cf0aff0'
SPEC = importlib.util.spec_from_file_location('presto_smoke', ROOT / 'scripts/engines/presto.py')
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)


def docker(*args, input=None, timeout=60):
    return subprocess.run(['docker', *args], input=input, text=True, capture_output=True,
                          timeout=timeout, check=True).stdout.strip()


def run(command=docker, verify=SMOKE.verify, pause=time.sleep):
    """Only remove the uniquely named container created by this invocation."""
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    command('pull', '--platform', 'linux/amd64', IMAGE, timeout=300)
    name = 'cube-presto-' + uuid.uuid4().hex
    owned = False
    try:
        command('create', '--name', name, '--platform', 'linux/amd64',
                '--publish', '127.0.0.1::8080', '--memory', '2g', IMAGE)
        owned = True
        with tempfile.TemporaryDirectory() as directory:
            catalog = Path(directory) / 'memory.properties'
            catalog.write_text('connector.name=memory\n')
            command('cp', str(catalog), name + ':/opt/presto-server/etc/catalog/memory.properties')
        command('start', name)
        ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
        port = ports['8080/tcp'][0]['HostPort']
        endpoint = 'http://127.0.0.1:' + str(int(port))
        # The CLI probes the actual query service, not merely an open HTTP port.
        for attempt in range(60):
            try:
                version = command('exec', name, 'presto-cli', '--execute', 'SELECT version()',
                                  '--output-format', 'CSV', timeout=5)
                if version != '"0.293"':
                    raise ValueError('unexpected engine version')
                break
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                if attempt == 59:
                    raise TimeoutError('Presto readiness failed')
                pause(1)
        command('exec', '-i', name, 'presto-cli', '--file', '/dev/stdin',
                input=(ROOT / 'scripts/engines/presto-fixture.sql').read_text(), timeout=30)
        verify(endpoint)
    finally:
        if owned:
            # Cleanup failure is itself a failed run, never a passing evidence line.
            command('rm', '--force', name, timeout=30)


def main():
    try:
        run()
    except (OSError, ValueError, KeyError, TypeError, IndexError,
            subprocess.SubprocessError, SMOKE.http.client.HTTPException) as error:
        print('check engines-container: FAILED — ' + type(error).__name__, file=sys.stderr)
        return 1
    print('check engines-container: PASSED — Presto 0.293, grouped sum golden, 1 case; not full T2 acceptance')
    return 0


if __name__ == '__main__':
    sys.exit(main())
