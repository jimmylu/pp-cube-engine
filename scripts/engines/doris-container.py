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
IMAGE = 'apache/doris@sha256:4d3aef8f4fe63f6e32b493b4fd7b2a682d19e2c800d1d812131e67aaba0d212a'
VERSION = '4.1.3'
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


def failure_point(error):
    """Stage, case name and numeric server code only; never a server message, SQL or endpoint."""
    name = str(PROGRESS.get('case', ''))
    code = getattr(error, 'errno', None)
    return ('stage=' + PROGRESS.get('stage', 'setup')
            + (' case=' + name if re.fullmatch(r'[A-Za-z0-9_]{1,64}', name) else '')
            + (' statement=%d' % PROGRESS['statement'] if PROGRESS.get('stage') == 'fixture' else '')
            + (' code=%d' % code if type(code) is int else ''))


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


def run(cases, command=docker, pause=time.sleep):
    import mysql.connector
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    print('Doris container: pulling pinned amd64 4.1.3 image', flush=True)
    command('pull', '--platform', 'linux/amd64', IMAGE, timeout=600)
    name = 'cube-doris-' + uuid.uuid4().hex
    try:
        command('create', '--name', name, '--platform', 'linux/amd64',
                '--publish', '127.0.0.1::9030', '--memory', '6g', IMAGE)
        command('start', name)
        ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
        port = int(ports['9030/tcp'][0]['HostPort'])
        print('Doris container: waiting for SQL readiness', flush=True)
        deadline = time.monotonic() + 300
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
            print('Doris container: executing live-compiled manifest', flush=True)
            suite(connection, cases)
        finally:
            connection.close()
    finally:
        print('Doris container: removing owned container', flush=True)
        command('rm', '--force', name, timeout=30)


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
    print('check doris-container: PASSED — Doris 4.1.3, %d live-compiled manifest cases (%s)'
          % (len(cases), MANIFEST.scenarios(cases)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
