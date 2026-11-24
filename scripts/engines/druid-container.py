#!/usr/bin/env python3
"""Provision one disposable, pinned Druid, execute the live-compiled manifest on it
and compare every case's SQL with pinned Cube Core's Druid dialect.

Druid's startup is the cost of this tier, so execution (B0-DRUID-004) and the Cube
Core comparison (B0-DRUID-003) share one engine. Nothing here is published.
"""
import importlib.util
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
# Druid 26.0.0, the version in production: its linux/amd64 manifest, not a mutable tag.
VERSION = '26.0.0'
IMAGE = 'apache/druid@sha256:9a3a07819058693b29bb5fc145e071c3b3a1f551960640529a60ad5198f29b59'
COMPARISON = 'B0-DRUID-003'
# What the cases need of the engine beyond the image's smallest example configuration.
# SQL-compatible NULLs: by default Druid 26 stores a NULL number as 0 and a NULL string
# as empty, and the expectations are those of the other engines.
COMMON = ('druid.generic.useDefaultValueForNull=false',)
# Two exact distinct counts in one query need three merge buffers; the example has two.
QUERYING = ('druid.processing.numMergeBuffers=6',)
SERVICES = ('coordinator-overlord', 'broker', 'router', 'historical', 'middleManager')
TYPES = {'text': 'VARCHAR', 'number': 'DOUBLE', 'bool': 'BOOLEAN'}

SPEC = importlib.util.spec_from_file_location('cube_core', ROOT / 'scripts/engines/cube-core.py')
CUBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CUBE)
MANIFEST = CUBE.MANIFEST

# Only locally authored, fixed messages may enter public failure evidence.
SAFE_FAILURES = CUBE.SAFE_FAILURES | frozenset({
    'unsupported execution manifest', 'malformed execution case', 'unsupported manifest parameter',
    'unsupported expected cell', 'unsupported fixture statement', 'HTTP redirects are not supported',
    'invalid query response', 'Druid returned a query error', 'unexpected result size',
    'result differs from the case expectation', 'unexpected engine version',
    'Druid did not become ready', 'ingestion task failed', 'ingestion task did not finish',
    'ingested segments were not served',
})
# Where a run stopped, for the fixed failure line: a stage and a public case name.
PROGRESS = {}


def failure(error):
    """A fixed category, the stage and the case; never SQL, an endpoint or a server message."""
    message = str(error)
    category = (message if type(error) in (ValueError, TimeoutError) and message in SAFE_FAILURES
                else type(error).__name__)
    name = str(PROGRESS.get('case', ''))
    return (category + ' at stage=' + PROGRESS.get('stage', 'setup')
            + (' case=' + name if re.fullmatch(r'[A-Za-z0-9_]{1,64}', name) else '')
            + (' statement=%d' % PROGRESS['statement'] if PROGRESS.get('stage') == 'fixture'
               and type(PROGRESS.get('statement')) is int else ''))


def docker(*args, input=None, timeout=60):
    result = subprocess.run(['docker', *args], input=input, text=True, capture_output=True,
                            timeout=timeout, check=True)
    return (result.stdout + (result.stderr if args[0] == 'logs' else '')).strip()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('HTTP redirects are not supported')


def call(endpoint, path, body=None, timeout=60):
    """One bounded request to the disposable engine; a refusal is a fixed category."""
    request = urllib.request.Request(
        endpoint + path, data=None if body is None else json.dumps(body).encode('utf-8'),
        headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=timeout) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        error.close()
        raise ValueError('Druid returned a query error') from None


def parameter(item):
    """A manifest parameter as Druid's SQL API takes it, apart from the statement."""
    kind, value = item.get('type'), item.get('value')
    if set(item) != {'type', 'value'} or kind not in TYPES:
        raise ValueError('unsupported manifest parameter')
    if ((kind == 'text' and isinstance(value, str))
            or (kind == 'bool' and isinstance(value, bool))
            or (kind == 'number' and isinstance(value, (int, float)) and not isinstance(value, bool)
                and math.isfinite(value))):
        return {'type': TYPES[kind], 'value': value}
    raise ValueError('unsupported manifest parameter')


def cell(value):
    """An expected cell in the form Druid's SQL API returns it."""
    if isinstance(value, dict):
        stamp = value.get('timestamp')
        if list(value) != ['timestamp'] or not isinstance(stamp, str) or len(stamp) != 19 or stamp[10] != 'T':
            raise ValueError('unsupported expected cell')
        return stamp + '.000Z'
    if isinstance(value, list):
        raise ValueError('unsupported expected cell')
    return value


def execute(endpoint, sql, timeout=60, parameters=(), session=None, ask=call):
    """The statement unchanged, its parameters apart, the manifest's session as query context."""
    body = {'query': sql, 'resultFormat': 'array', 'header': True, 'context': dict(session or {})}
    if parameters:
        body['parameters'] = [parameter(item) for item in parameters]
    result = ask(endpoint, '/druid/v2/sql', body, timeout)
    if (not isinstance(result, list) or not result
            or any(not isinstance(row, list) for row in result)):
        raise ValueError('invalid query response')
    if len(result) > 1001:
        raise ValueError('unexpected result size')
    return result[0], result[1:]


def segments(endpoint, table, ask=call):
    result = ask(endpoint, '/druid/v2/sql', {
        'query': 'SELECT "version", is_available, is_overshadowed FROM sys.segments WHERE datasource = ?',
        'parameters': [{'type': 'VARCHAR', 'value': table}], 'resultFormat': 'array'})
    if not isinstance(result, list):
        raise ValueError('invalid query response')
    return result


def load(endpoint, statement, ask=call, pause=time.sleep, attempts=300):
    """One fixture statement: a SQL ingestion that replaces a whole datasource.

    It is loaded when a segment version that did not exist before is served and no
    older one is. Counting rows instead would pass on the data of the fixture before."""
    match = re.match(r'REPLACE INTO "([a-z][a-z0-9_]*)" OVERWRITE ALL ', statement)
    if not match:
        raise ValueError('unsupported fixture statement')
    table = match.group(1)
    before = {row[0] for row in segments(endpoint, table, ask)}
    task = ask(endpoint, '/druid/v2/sql/task', {'query': statement})
    identifier = task.get('taskId') if isinstance(task, dict) else None
    if not isinstance(identifier, str) or not re.fullmatch(r'[A-Za-z0-9_-]+', identifier):
        raise ValueError('invalid query response')
    for attempt in range(attempts):
        try:
            status = ask(endpoint, '/druid/indexer/v1/task/' + identifier + '/status')['status']['status']
        except (KeyError, TypeError):
            raise ValueError('invalid query response') from None
        if status == 'SUCCESS':
            break
        if status == 'FAILED':
            raise ValueError('ingestion task failed')
        if attempt == attempts - 1:
            raise TimeoutError('ingestion task did not finish')
        pause(1)
    for attempt in range(attempts):
        rows = segments(endpoint, table, ask)
        if (any(row[0] not in before and row[1] == 1 for row in rows)
                and not any(row[0] in before and row[1] == 1 and row[2] == 0 for row in rows)):
            return
        pause(1)
    raise TimeoutError('ingested segments were not served')


def suite(endpoint, cases, session, cube=None, run=execute, fixture=load, ask=None):
    """Every case: loaded fixture, the kernel's SQL returns the expected rows and, with
    Cube Core, Cube's SQL and parameters equal the kernel's but for its default limit."""
    ask = ask or CUBE.cube_sql
    loaded = None
    for case in cases:
        PROGRESS.update(stage='fixture', case=case['name'])
        if case['fixture'] != loaded:
            for index, statement in enumerate(case['fixture'], 1):
                PROGRESS['statement'] = index
                fixture(endpoint, statement)
            loaded = case['fixture']
        if cube:
            PROGRESS['stage'] = 'cube-core-sql'
            sql, params = ask(cube, CUBE.cube_query(case))
            if sql != CUBE.expected_sql(case):
                raise ValueError('Cube Core SQL differs from the kernel')
            if params != [item.get('value') for item in case['params']]:
                raise ValueError('Cube Core parameters differ from the kernel')
        PROGRESS['stage'] = 'execution'
        columns, rows = run(endpoint, case['sql'], 60, tuple(case['params']), session)
        if columns != case['columns'] or rows != [[cell(value) for value in row] for row in case['rows']]:
            raise ValueError('result differs from the case expectation')


def script():
    """Every Druid service and its ZooKeeper in the one container, by the image's own scripts."""
    conf = 'conf/druid/single-server/nano-quickstart'
    lines = ['cd /opt/druid', 'export DRUID_LOG_DIR=/tmp/log']
    lines += ["printf '\\n%s\\n' >> %s/_common/common.runtime.properties" % (line, conf) for line in COMMON]
    lines += ["printf '\\n%s\\n' >> %s/%s/runtime.properties" % (line, conf, service)
              for line in QUERYING for service in ('broker', 'historical')]
    lines += ['bin/run-zk conf > /tmp/zk.out 2>&1 &']
    lines += ['bin/run-druid %s %s > /tmp/%s.out 2>&1 &' % (service, conf, service) for service in SERVICES]
    return '\n'.join(lines + ['wait'])


def ready(endpoint, ask=call, pause=time.sleep, attempts=120):
    """The router answers SQL through the broker, and reports the pinned version."""
    for attempt in range(attempts):
        try:
            if ask(endpoint, '/druid/v2/sql', {'query': 'SELECT 1 AS probe'}, 5) == [{'probe': 1}]:
                break
        except (OSError, ValueError):
            pass
        if attempt == attempts - 1:
            raise TimeoutError('Druid did not become ready')
        pause(5)
    status = ask(endpoint, '/status', None, 5)
    if not isinstance(status, dict) or status.get('version') != VERSION:
        raise ValueError('unexpected engine version')


def run(cases, session, model=None, data_sources=(), command=docker, verify=suite, await_ready=ready,
        start_cube=None):
    """Only remove the uniquely named containers created by this invocation."""
    start_cube = start_cube or CUBE.start_cube
    PROGRESS.update(stage='provision')
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    print('Druid container: pulling pinned amd64 image', flush=True)
    command('pull', '--platform', 'linux/amd64', IMAGE, timeout=900)
    if model is not None:
        print('Druid container: pulling pinned Cube Core ' + CUBE.CUBE_VERSION + ' image', flush=True)
        command('pull', '--platform', 'linux/amd64', CUBE.CUBE_IMAGE, timeout=900)
    name = 'cube-druid-' + uuid.uuid4().hex
    owned, cube_name = False, None
    try:
        with tempfile.TemporaryDirectory() as directory:
            owned = True
            command('create', '--name', name, '--platform', 'linux/amd64',
                    '--publish', '127.0.0.1::8888', '--memory', '6g',
                    '--entrypoint', 'bash', IMAGE, '-c', script())
            print('Druid container: starting engine', flush=True)
            command('start', name)
            ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
            endpoint = 'http://127.0.0.1:' + str(int(ports['8888/tcp'][0]['HostPort']))
            cube = None
            if model is not None:
                PROGRESS['stage'] = 'cube-core'
                Path(directory).chmod(0o755)
                file = Path(directory) / 'model.yml'
                file.write_text(model)
                file.chmod(0o644)
                print('Druid container: starting Cube Core', flush=True)
                # Cube needs a dialect to write SQL; it never connects to the engine.
                cube_name, cube = start_cube(command, directory, list(data_sources), database={
                    'DB_TYPE': 'druid', 'DB_URL': 'http://127.0.0.1:8888', 'DB_USER': 'cube'})
            PROGRESS['stage'] = 'readiness-version'
            print('Druid container: waiting for query readiness and exact version', flush=True)
            await_ready(endpoint)
            print('Druid container: executing live-compiled manifest', flush=True)
            verify(endpoint, cases, session, cube)
    finally:
        for container in (cube_name, name if owned else None):
            if container:
                # Cleanup failure is itself a failed run, never a passing evidence line.
                print('Druid container: removing owned container', flush=True)
                command('rm', '--force', container, timeout=30)


def main():
    compare = '--no-cube-core' not in sys.argv[1:]
    try:
        # Compile before provisioning, so a compiler failure never starts an engine.
        print('Druid container: compiling execution manifest', flush=True)
        if compare:
            CUBE.pinned()
            cases, session, model, data_sources = MANIFEST.compiled_sources('druid')
        else:
            (cases, session), model, data_sources = MANIFEST.compiled_manifest('druid'), None, ()
        run(cases, session, model, data_sources)
    except Exception as error:
        print('check druid-container: FAILED — ' + failure(error), file=sys.stderr)
        return 1
    print('check druid-container: PASSED — Druid %s, %d live-compiled manifest cases (%s)%s'
          % (VERSION, len(cases), MANIFEST.scenarios(cases),
             '; Cube Core %s Druid SQL and parameters equal the kernel\'s but for Cube\'s default '
             'row limit (%s)' % (CUBE.CUBE_VERSION, COMPARISON) if compare else
             '; Cube Core comparison not run'))
    return 0


if __name__ == '__main__':
    sys.exit(main())
