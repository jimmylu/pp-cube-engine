#!/usr/bin/env python3
"""Disposable Doris execution of the live-compiled conformance manifest."""
import datetime
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
# Doris 2.1.8, the version in production: the linux/amd64 manifests of the
# published frontend and backend images. 2.1 has no single-container image.
VERSION = '2.1.8'
FE_IMAGE = 'apache/doris@sha256:c190478d0f5c13fe0b3ba13f7cf384b022b586b41589348960b748aef7a99dbe'
BE_IMAGE = 'apache/doris@sha256:a1f31b331b7fe27f7c1be2b5b134a8c8fc98f1c25f5bb3aeeaf0579e33486d46'
SPEC = importlib.util.spec_from_file_location('presto_container', ROOT / 'scripts/engines/presto-container.py')
PRESTO = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRESTO)
docker = PRESTO.docker
MANIFEST_SPEC = importlib.util.spec_from_file_location('engine_manifest', ROOT / 'scripts/engines/manifest.py')
MANIFEST = importlib.util.module_from_spec(MANIFEST_SPEC)
MANIFEST_SPEC.loader.exec_module(MANIFEST)


def verify(cursor, sql, params, columns, rows):
    """Driver sends parameters separately through COM_STMT_EXECUTE."""
    cursor.execute(sql, params)
    actual = cursor.fetchall()
    if list(cursor.column_names) != columns or actual != rows:
        raise ValueError('Doris result differs from fixture expectation')


def compiled_cases(emit=subprocess.run):
    """The Doris manifest; this tier sets no session property, so it accepts none."""
    cases, session = MANIFEST.compiled_manifest('doris', emit)
    if session:
        raise ValueError('unsupported execution manifest')
    return cases


def parameter(item):
    kind = item.get('type')
    if kind == 'null' and set(item) == {'type'}:
        return None
    expected = {'text': str, 'number': (int, float), 'bool': bool}.get(kind)
    value = item.get('value')
    if expected is None or not isinstance(value, expected) or (kind == 'number' and isinstance(value, bool)):
        raise ValueError('unsupported manifest parameter')
    return value


def cell(value):
    if isinstance(value, dict):
        if list(value) != ['timestamp']:
            raise ValueError('unsupported expected cell')
        return datetime.datetime.fromisoformat(value['timestamp'])
    if isinstance(value, list):
        raise ValueError('unsupported expected cell')
    return value


# Where a run stopped, for the fixed failure line: a stage and a public case name.
PROGRESS = {}


# Fixed phrases a server message may be reduced to; the message itself is never printed.
PHRASES = ('backend', 'replica', 'storage medium', 'capacity', 'tablet', 'timeout', 'not found',
           'already exists', 'syntax')
# A backend can report alive before its storage is registered; table creation then
# fails with the generic code and one of these phrases until the next heartbeat.
STORAGE_NOT_READY = ('backend', 'replica', 'storage medium', 'capacity')


def phrases(error):
    message = str(getattr(error, 'msg', '') or '').lower()
    return [phrase for phrase in PHRASES if phrase in message]


def await_storage(connection, pause=time.sleep, clock=time.monotonic, limit=120):
    """Alive backends are not enough: wait until a one-replica table can really be created."""
    PROGRESS.update(stage='storage-readiness')
    deadline = clock() + limit
    cursor = connection.cursor()
    try:
        cursor.execute('CREATE DATABASE IF NOT EXISTS cube_readiness')
        while True:
            try:
                cursor.execute("CREATE TABLE IF NOT EXISTS cube_readiness.probe (value INT) DUPLICATE KEY(value) "
                               "DISTRIBUTED BY HASH(value) BUCKETS 1 PROPERTIES ('replication_num'='1')")
                break
            except Exception as error:
                # Only the known not-yet-registered condition is retried; anything else is a failure.
                if getattr(error, 'errno', None) != 1105 or not set(phrases(error)) & set(STORAGE_NOT_READY):
                    raise
                if clock() >= deadline:
                    raise TimeoutError('Doris storage not ready') from None
                pause(2)
        cursor.execute('DROP DATABASE cube_readiness FORCE')
    finally:
        cursor.close()


def failure_point(error):
    """Stage, case name, numeric server code and fixed phrases only; never a server message,
    SQL or endpoint."""
    name = str(PROGRESS.get('case', ''))
    code = getattr(error, 'errno', None)
    return ('stage=' + PROGRESS.get('stage', 'setup')
            + (' case=' + name if PROGRESS.get('stage') in ('fixture', 'case')
               and re.fullmatch(r'[A-Za-z0-9_]{1,64}', name) else '')
            + (' statement=%d' % PROGRESS['statement'] if PROGRESS.get('stage') == 'fixture' else '')
            + (' code=%d' % code if type(code) is int else '')
            + (' phrases=' + ','.join(phrases(error)) if phrases(error) else ''))


def suite(connection, cases):
    """Runs every live-compiled case; each fixture leaves exactly its own data."""
    loaded = None
    for case in cases:
        PROGRESS.update(stage='fixture', case=case['name'])
        if case['fixture'] != loaded:
            cursor = connection.cursor()
            for index, statement in enumerate(case['fixture'], 1):
                PROGRESS['statement'] = index
                cursor.execute(statement)
            cursor.close()
            loaded = case['fixture']
        PROGRESS['stage'] = 'case'
        # Parameters travel separately through the native prepared protocol.
        cursor = connection.cursor(prepared=True)
        try:
            verify(cursor, case['sql'], tuple(parameter(item) for item in case['params']),
                   case['columns'], [tuple(cell(value) for value in row) for row in case['rows']])
        finally:
            cursor.close()


def provision(command, suffix, attempts=8):
    """One frontend and one backend on a network of their own. The images take the
    frontend's address as an IP, so the network has a subnet chosen here; a subnet
    already in use on this host is skipped. Returns what was created, to remove."""
    owned = []
    try:
        for attempt in range(attempts):
            # 172.28.0.0/14 is private space Docker's defaults leave alone.
            third = (int(suffix[:6], 16) + attempt * 37) % 250
            base = '172.%d.%d.' % (28 + (int(suffix[6:8], 16) % 4), third)
            network = 'cube-doris-' + suffix
            try:
                command('network', 'create', '--subnet', base + '0/24', network)
                break
            except subprocess.CalledProcessError:
                if attempt == attempts - 1:
                    raise
        owned.append(('network', network))
        servers = 'fe1:' + base + '2:9010'
        frontend, backend = 'cube-doris-fe-' + suffix, 'cube-doris-be-' + suffix
        command('create', '--name', frontend, '--platform', 'linux/amd64', '--network', network,
                '--ip', base + '2', '--env', 'FE_SERVERS=' + servers, '--env', 'FE_ID=1',
                '--publish', '127.0.0.1::9030', '--memory', '3g', FE_IMAGE)
        owned.append(('container', frontend))
        command('create', '--name', backend, '--platform', 'linux/amd64', '--network', network,
                '--ip', base + '3', '--env', 'FE_SERVERS=' + servers,
                '--env', 'BE_ADDR=' + base + '3:9050', '--env', 'SKIP_CHECK_ULIMIT=true',
                '--memory', '4g', BE_IMAGE)
        owned.append(('container', backend))
        command('start', frontend)
        command('start', backend)
        return owned, frontend
    except Exception:
        release(command, owned)
        raise


def release(command, owned):
    """Removes exactly what this invocation created, containers before their network."""
    for kind, name in reversed(owned):
        if kind == 'container':
            command('rm', '--force', name, timeout=30)
        else:
            command('network', 'rm', name, timeout=30)


def run(cases, command=docker, pause=time.sleep):
    import mysql.connector
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    print('Doris container: pulling pinned amd64 %s frontend and backend images' % VERSION, flush=True)
    command('pull', '--platform', 'linux/amd64', FE_IMAGE, timeout=900)
    command('pull', '--platform', 'linux/amd64', BE_IMAGE, timeout=1800)
    owned, name = provision(command, uuid.uuid4().hex)
    try:
        ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
        port = int(ports['9030/tcp'][0]['HostPort'])
        print('Doris container: waiting for SQL readiness', flush=True)
        deadline = time.monotonic() + 600
        while True:
            try:
                connection = mysql.connector.connect(host='127.0.0.1', port=port, user='root',
                                                     connection_timeout=5, read_timeout=15,
                                                     write_timeout=15, use_pure=True)
                cursor = connection.cursor()
                cursor.execute('SELECT VERSION()')
                cursor.fetchall()
                cursor.execute('SHOW BACKENDS')
                backends = [dict(zip(cursor.column_names, row)) for row in cursor.fetchall()]
                if not backends or not all(str(row['Alive']).lower() == 'true' for row in backends):
                    cursor.close()
                    connection.close()
                    if time.monotonic() >= deadline:
                        raise TimeoutError('Doris backends not ready')
                    pause(2)
                    continue
                versions = [row['Version'] for row in backends]
                cursor.close()
                break
            except mysql.connector.Error:
                if 'connection' in locals():
                    connection.close()
                if time.monotonic() >= deadline:
                    raise TimeoutError('Doris SQL readiness timed out') from None
                pause(2)
        try:
            if not all(version.startswith('doris-' + VERSION + '-') for version in versions):
                raise ValueError('unexpected Doris runtime version')
            print('Doris container: waiting for backend storage', flush=True)
            await_storage(connection, pause)
            print('Doris container: executing live-compiled manifest', flush=True)
            suite(connection, cases)
        finally:
            connection.close()
    finally:
        print('Doris container: removing owned containers and network', flush=True)
        release(command, owned)


def main():
    try:
        # Compile before provisioning, so a compiler failure never starts an engine.
        print('Doris container: compiling execution manifest', flush=True)
        cases = compiled_cases()
        run(cases)
    except Exception as error:
        # Never publish endpoints, driver/server responses or business input.
        print('check doris-container: FAILED — ' + type(error).__name__ + ' at ' + failure_point(error),
              file=sys.stderr)
        return 1
    print('check doris-container: PASSED — Doris %s, %d live-compiled manifest cases (%s)'
          % (VERSION, len(cases), MANIFEST.scenarios(cases)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
