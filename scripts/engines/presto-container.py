#!/usr/bin/env python3
"""Provision one disposable, pinned Presto engine and execute the live-compiled manifest."""
import importlib.util
import json
import re
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
# Presto 0.293 linux/amd64 manifest, not a mutable tag.
# Runtime build identity observed for this exact manifest in real-engine CI.
VERSION = '0.293-2f48ff6'
IMAGE = 'prestodb/presto@sha256:6e7f7b4470522f5599a3b42e85036a526256d51b532fe8e6098d88ff8cf0aff0'
SPEC = importlib.util.spec_from_file_location('presto_smoke', ROOT / 'scripts/engines/presto.py')
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)


def docker(*args, input=None, timeout=60):
    result = subprocess.run(['docker', *args], input=input, text=True, capture_output=True,
                            timeout=timeout, check=True)
    return (result.stdout + (result.stderr if args[0] == 'logs' else '')).strip()


def run(cases, session=None, command=docker, verify=SMOKE.verify_manifest, pause=time.sleep):
    """Only remove the uniquely named container created by this invocation."""
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    print('Presto container: pulling pinned amd64 image', flush=True)
    command('pull', '--platform', 'linux/amd64', IMAGE, timeout=300)
    name = 'cube-presto-' + uuid.uuid4().hex
    owned = False
    stage = 'create'
    try:
        owned = True
        command('create', '--name', name, '--platform', 'linux/amd64',
                '--publish', '127.0.0.1::8080', '--memory', '2g', IMAGE)
        with tempfile.TemporaryDirectory() as directory:
            catalog = Path(directory) / 'memory.properties'
            catalog.write_text('connector.name=memory\n')
            command('cp', str(catalog), name + ':/opt/presto-server/etc/catalog/memory.properties')
        print('Presto container: starting engine', flush=True)
        command('start', name)
        ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
        port = ports['8080/tcp'][0]['HostPort']
        endpoint = 'http://127.0.0.1:' + str(int(port))
        stage = 'readiness-version'
        print('Presto container: waiting for query readiness and exact version', flush=True)
        # The CLI probes the actual query service, not merely an open HTTP port.
        for attempt in range(60):
            try:
                version = command('exec', name, '/opt/presto-cli', '--execute', 'SELECT DISTINCT node_version FROM system.runtime.nodes',
                                  '--output-format', 'CSV', timeout=5)
                if version != json.dumps(VERSION):
                    # Permit only version-shaped output, never arbitrary CLI text.
                    reported = version if re.fullmatch(r'"[0-9]+(?:[.][0-9]+)*(?:-[A-Za-z0-9]+)*"', version) else 'unrecognized-format'
                    print('engine reported version: ' + reported, file=sys.stderr)
                    raise ValueError('unexpected engine version')
                break
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as probe_error:
                if attempt == 59:
                    raw = str(getattr(probe_error, 'stderr', '') or '')
                    # Probe SQL and this disposable engine contain no business input.
                    # Retain only known fixed diagnostic phrases, never raw messages.
                    phrases = ['Connection refused', 'not registered', 'not found',
                               'No such file', 'Exec format error', 'Permission denied',
                               'UnsupportedClassVersionError', 'Server starting',
                               'Initializing', 'Unable to access', 'Unknown option',
                               'Catalog must be specified', 'Schema must be specified']
                    print('probe categories: ' + ','.join(p for p in phrases if p.lower() in raw.lower()) +
                          '; status=' + str(getattr(probe_error, 'returncode', 'timeout')), file=sys.stderr)
                    state = json.loads(command('inspect', '--format', '{{json .State}}', name))
                    print('engine state: ' + str(state.get('Status')) +
                          '; exit=' + str(state.get('ExitCode')) +
                          '; oom=' + str(state.get('OOMKilled')), file=sys.stderr)
                    logs = command('logs', name)
                    # Only exception class names; never hostnames or raw server messages.
                    classes = sorted(set(re.findall(r'\b[A-Za-z.]+(?:Exception|Error)\b', logs)))
                    print('engine error classes: ' + ','.join(classes), file=sys.stderr)
                    raise TimeoutError('Presto readiness failed')
                pause(1)
        stage = 'memory-readiness'
        print('Presto container: waiting for Memory worker execution', flush=True)
        for attempt in range(60):
            # A version query can pass before connector workers are advertised.
            # Unique probe tables also isolate partial CTAS failures per attempt.
            table = 'memory.default.cube_readiness_' + str(attempt)
            try:
                command('exec', name, '/opt/presto-cli', '--execute',
                        'CREATE TABLE ' + table + ' AS SELECT 1 AS value', timeout=5)
                break
            except subprocess.CalledProcessError as error:
                raw = (error.stdout or '') + (error.stderr or '')
                if not any(phrase in raw for phrase in ('No nodes available', 'No Memory nodes available')):
                    raise
                if attempt == 59:
                    raise TimeoutError('Memory workers did not become ready') from None
                pause(1)
        command('exec', name, '/opt/presto-cli', '--execute', 'DROP TABLE ' + table, timeout=5)
        stage = 'manifest-execution'
        print('Presto container: executing live-compiled manifest over HTTP', flush=True)
        verify(endpoint, cases, session)
    except Exception:
        print('engine failure stage: ' + stage, file=sys.stderr)
        raise
    finally:
        if owned:
            # Cleanup failure is itself a failed run, never a passing evidence line.
            print('Presto container: removing owned container', flush=True)
            command('rm', '--force', name, timeout=30)


def main():
    try:
        # Compile before provisioning, so a compiler failure never starts an engine.
        print('Presto container: compiling execution manifest', flush=True)
        cases, session = SMOKE.MANIFEST.compiled_manifest('presto')
        run(cases, session)
    except (OSError, ValueError, KeyError, TypeError, IndexError,
            subprocess.SubprocessError, SMOKE.http.client.HTTPException) as error:
        print('check engines-container: FAILED — ' + SMOKE.failure_category(error), file=sys.stderr)
        return 1
    print('check engines-container: PASSED — Presto 0.293, %d live-compiled manifest cases (%s)'
          % (len(cases), SMOKE.MANIFEST.scenarios(cases)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
