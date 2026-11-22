#!/usr/bin/env python3
"""Disposable Doris execution of stored goldens and the live-compiled manifest; not full T2."""
import datetime
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
IMAGE = 'apache/doris@sha256:4d3aef8f4fe63f6e32b493b4fd7b2a682d19e2c800d1d812131e67aaba0d212a'
VERSION = '4.1.3'
SNAPS = ROOT / 'crates/pp-cube-compile/tests/snapshots'
SPEC = importlib.util.spec_from_file_location('presto_container', ROOT / 'scripts/engines/presto-container.py')
PRESTO = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRESTO)
docker = PRESTO.docker
EMITTER = ('cargo', 'run', '-q', '--locked', '-p', 'pp-cube-conformance',
           '--bin', 'execution-cases', '--', 'doris')
CASE_KEYS = {'name', 'scenario', 'fixture', 'sql', 'params', 'columns', 'rows'}


def golden(name):
    text = (SNAPS / ('doris__' + name + '.snap')).read_text()
    sections = text.split('---\n', 2)
    if len(sections) != 3 or sections[0]:
        raise ValueError('invalid snapshot header')
    return sections[2].strip()


def verify(cursor, sql, params, columns, rows):
    """Driver sends parameters separately through COM_STMT_EXECUTE."""
    cursor.execute(sql, params)
    actual = cursor.fetchall()
    if list(cursor.column_names) != columns or actual != rows:
        raise ValueError('Doris result differs from fixture expectation')


def suite(connection):
    cursor = connection.cursor()
    fixture = (ROOT / 'scripts/engines/doris-fixture.sql').read_text()
    for statement in fixture.split(';'):
        if statement.strip():
            cursor.execute(statement)
    cursor.close()
    cursor = connection.cursor(prepared=True)
    try:
        verify(cursor, golden('basic_doris_sql_and_identity'), ('north', 0),
               ['sales__region', 'sales__count', 'sales__total', 'sales__average',
                'sales__minimum', 'sales__maximum', 'sales__unique'], [])
        # The golden OFFSET 2 must discard its single group; also check nonempty aggregates.
        sql = golden('basic_doris_sql_and_identity').replace('OFFSET 2', 'OFFSET 0')
        verify(cursor, sql, ('north', 0),
               ['sales__region', 'sales__count', 'sales__total', 'sales__average',
                'sales__minimum', 'sales__maximum', 'sales__unique'],
               [('north', 2, 30.0, 15.0, 10.0, 20.0, 2)])
        sql = golden('doris_join_grouped').split('\n-- parameters: ')[0]
        verify(cursor, sql, ('central', 'north'), ['stores__store_name', 'sales__amount'],
               [('central', 30.0)])
        blocks = golden('doris_literal_patterns').split('\n\n')
        if len(blocks) != 6:
            raise ValueError('unexpected pattern snapshot shape')
        for block, expected in zip(blocks, (5.0, None, 5.0, None, 5.0, None)):
            sql = block.split(':\n', 1)[1].rsplit('\n[Text(', 1)[0]
            verify(cursor, sql, ('A\\%\\_\\\\B', 'a%_\\b'), ['sales__total'], [(expected,)])
        # Wildcard-looking input must not match this nonliteral decoy.
        for block, expected in zip(blocks, (None, 99.0, None, 99.0, None, 99.0)):
            sql = block.split(':\n', 1)[1].rsplit('\n[Text(', 1)[0]
            verify(cursor, sql, ('A\\%\\_\\\\B', 'axxb'), ['sales__total'], [(expected,)])
        # Negative patterns also execute on a nonmatching row.
        for block in blocks[1::2]:
            sql = block.split(':\n', 1)[1].rsplit('\n[Text(', 1)[0]
            verify(cursor, sql, ('A\\%\\_\\\\B', 'north'), ['sales__total'], [(30.0,)])
        sql = golden('doris_security_pattern_conjoins_user_or').rsplit('\n[Text(', 1)[0]
        verify(cursor, sql, ('central\\%', 'west', 'north\\_'), ['sales__total'], [(18.0,)])
    finally:
        cursor.close()


def compiled_cases(emit=subprocess.run):
    """Compiles the conformance cases now; the kernel itself never executes them."""
    output = emit(EMITTER, cwd=ROOT, text=True, stdout=subprocess.PIPE, timeout=900, check=True)
    manifest = json.loads(output.stdout)
    cases = manifest.get('cases')
    if manifest.get('schema') != 1 or manifest.get('dialect') != 'doris' or not cases:
        raise ValueError('unsupported execution manifest')
    if any(set(case) != CASE_KEYS or not case['fixture'] for case in cases):
        raise ValueError('malformed execution case')
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


def manifest_suite(connection, cases):
    """Runs every live-compiled case on the engine the stored goldens just used."""
    cursor = connection.cursor()
    # Case fixtures create their tables only when absent; remove the stored-golden schema first.
    cursor.execute('DROP DATABASE IF EXISTS dw FORCE')
    cursor.close()
    loaded = None
    for case in cases:
        if case['fixture'] != loaded:
            cursor = connection.cursor()
            for statement in case['fixture']:
                cursor.execute(statement)
            cursor.close()
            loaded = case['fixture']
        # Parameters travel separately through the native prepared protocol.
        cursor = connection.cursor(prepared=True)
        try:
            verify(cursor, case['sql'], tuple(parameter(item) for item in case['params']),
                   case['columns'], [tuple(cell(value) for value in row) for row in case['rows']])
        finally:
            cursor.close()


def run(command=docker, pause=time.sleep, cases=None):
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
            print('Doris container: loading fixture and executing goldens', flush=True)
            suite(connection)
            if cases is not None:
                print('Doris container: executing live-compiled manifest', flush=True)
                manifest_suite(connection, cases)
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
        run(cases=cases)
    except Exception as error:
        # Never publish endpoints, driver/server responses or business input.
        print('check doris-container: FAILED — ' + type(error).__name__, file=sys.stderr)
        return 1
    print('check doris-container: PASSED — Doris 4.1.3, 19 stored-golden assertions and %d live-compiled manifest cases; '
          'not full T2 acceptance' % len(cases))
    return 0


if __name__ == '__main__':
    sys.exit(main())
