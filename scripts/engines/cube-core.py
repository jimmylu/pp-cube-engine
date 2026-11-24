#!/usr/bin/env python3
"""Tier T1: the kernel's Presto SQL compared with pinned Cube Core.

For every execution case, Cube Core at the release of the upstream pin is asked
for the SQL of the same model and query. That SQL and its parameters must equal
the kernel's, but for the default row limit Cube's API adds, and it must return
the case's expected rows on a disposable Presto. Cube Core runs in development
mode and never connects to a database; nothing here is published.
"""
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
# The release whose tag is the commit of the upstream pin. Move all three together
# with the pin: the comparison is only meaningful against the same Cube.
CUBE_VERSION = '1.8.28'
CUBE_COMMIT = '486f0d26bab58ab4fe0be0e4cdd39818b35a7870'
CUBE_IMAGE = 'cubejs/cube@sha256:e819c88b72fa2840a9914ded4ebd4ad4b9ef98eb9b2735c0fd46d87edb8b3f3a'
# Cube's API gives a query without a limit this one; the kernel leaves it to the caller.
DEFAULT_LIMIT = 10000
SCENARIO = 'B0-PRESTO-002'

SPEC = importlib.util.spec_from_file_location('presto_container', ROOT / 'scripts/engines/presto-container.py')
PRESTO = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRESTO)
SMOKE = PRESTO.SMOKE
MANIFEST = SMOKE.MANIFEST

SAFE_FAILURES = frozenset({
    'the Cube Core release does not match the upstream pin',
    'Cube Core did not become ready', 'unexpected Cube Core version',
    'invalid Cube Core response', 'unsupported Cube Core parameter',
    'Cube Core SQL differs from the kernel', 'Cube Core parameters differ from the kernel',
    'Cube Core result differs from the case expectation',
})
# Where a run stopped, for the fixed failure line: a stage and a public case name.
PROGRESS = {}


def failure(error):
    """A fixed category and the case; never SQL, an endpoint or a server message."""
    message = str(error)
    category = (message if type(error) is ValueError and message in SAFE_FAILURES
                else SMOKE.failure_category(error))
    name = str(PROGRESS.get('case', ''))
    return (category + ' at stage=' + PROGRESS.get('stage', 'setup')
            + (' case=' + name if re.fullmatch(r'[A-Za-z0-9_]{1,64}', name) else ''))


def pinned(read=subprocess.run):
    """The image is the release of the commit this repository builds against."""
    listed = read(['git', 'ls-files', '-s', 'vendor/cube'], cwd=ROOT, text=True,
                  stdout=subprocess.PIPE, check=True).stdout.split()
    if len(listed) < 2 or listed[1] != CUBE_COMMIT:
        raise ValueError('the Cube Core release does not match the upstream pin')


def cube_query(case):
    """The case as Cube's REST API takes it: its effective timezone stated, and its
    security filters appended as top-level filters, which is where the kernel puts them."""
    query = dict(case['query'])
    query.setdefault('timezone', case['timezone'])
    if case['security_filters']:
        query['filters'] = list(query.get('filters', [])) + list(case['security_filters'])
    return query


def expected_sql(case):
    """The kernel's SQL as Cube Core must give it: with Cube's default row limit when
    the query sets none. Any other difference is a failure."""
    if 'limit' in case['query']:
        return case['sql']
    return case['sql'] + '\nLIMIT %d' % DEFAULT_LIMIT


def cube_sql(endpoint, query, timeout=60, opener=urllib.request.urlopen):
    url = endpoint + '/cubejs-api/v1/sql?' + urllib.parse.urlencode({'query': json.dumps(query)})
    with opener(url, timeout=timeout) as response:
        document = json.load(response)
    try:
        sql, params = document['sql']['sql']
    except (KeyError, TypeError, ValueError):
        raise ValueError('invalid Cube Core response') from None
    if not isinstance(sql, str) or not isinstance(params, list):
        raise ValueError('invalid Cube Core response')
    return sql, params


def compare(endpoint, cube, cases, session, timeout=30, ask=cube_sql, execute=None):
    """Every case: Cube Core's SQL equals the kernel's, and returns the expected rows."""
    execute = execute or SMOKE.execute
    loaded = None
    for case in cases:
        PROGRESS.update(stage='fixture', case=case['name'])
        if case['fixture'] != loaded:
            for statement in case['fixture']:
                execute(endpoint, statement, timeout, (), session)
            loaded = case['fixture']
        PROGRESS['stage'] = 'cube-core-sql'
        sql, params = ask(cube, cube_query(case))
        if sql != expected_sql(case):
            raise ValueError('Cube Core SQL differs from the kernel')
        if any(not isinstance(value, str) for value in params):
            raise ValueError('unsupported Cube Core parameter')
        if params != [item.get('value') for item in case['params']]:
            raise ValueError('Cube Core parameters differ from the kernel')
        PROGRESS['stage'] = 'execution'
        bound = tuple({'type': 'text', 'value': value} for value in params)
        columns, rows = execute(endpoint, sql, timeout, bound, session)
        if columns != case['columns'] or rows != [[SMOKE.cell(value) for value in row] for row in case['rows']]:
            raise ValueError('Cube Core result differs from the case expectation')


# Cube needs a database type to choose a dialect; these settings are never connected to.
PRESTO_DATABASE = {'DB_TYPE': 'prestodb', 'DB_HOST': '127.0.0.1', 'DB_PORT': '8080',
                   'DB_USER': 'cube', 'DB_PRESTO_CATALOG': 'memory', 'DB_SCHEMA': 'dw'}


def start_cube(command, directory, data_sources, pause=time.sleep, opener=urllib.request.urlopen,
               database=None):
    """One disposable Cube Core in development mode. It is given connection settings
    for every data source because it needs a dialect to write SQL; it never connects."""
    name = 'cube-core-' + uuid.uuid4().hex
    settings = {'CUBEJS_DEV_MODE': 'true', 'CUBEJS_TELEMETRY': 'false',
                'CUBEJS_DATASOURCES': ','.join(['default'] + data_sources)}
    for prefix in ['CUBEJS_'] + ['CUBEJS_DS_' + source.upper() + '_' for source in data_sources]:
        settings.update({prefix + key: value for key, value in (database or PRESTO_DATABASE).items()})
    arguments = ['create', '--name', name, '--platform', 'linux/amd64',
                 '--publish', '127.0.0.1::4000', '--memory', '2g',
                 '--volume', str(directory) + ':/cube/conf/model:ro']
    for key, value in sorted(settings.items()):
        arguments += ['--env', key + '=' + value]
    command(*arguments, CUBE_IMAGE)
    command('start', name)
    ports = json.loads(command('inspect', '--format', '{{json .NetworkSettings.Ports}}', name))
    endpoint = 'http://127.0.0.1:' + str(int(ports['4000/tcp'][0]['HostPort']))
    for attempt in range(120):
        try:
            with opener(endpoint + '/cubejs-api/v1/meta', timeout=5) as response:
                json.load(response)
            break
        except (OSError, ValueError):
            if attempt == 119:
                raise TimeoutError('Cube Core did not become ready') from None
            pause(1)
    if ('Cube API server (' + CUBE_VERSION + ')') not in command('logs', name):
        raise ValueError('unexpected Cube Core version')
    return name, endpoint


def run(cases, session, model, data_sources, command=None, presto=None):
    command = command or PRESTO.docker
    presto = presto or PRESTO.run
    PROGRESS.update(stage='cube-core')
    command('info', '--format', '{{.ServerVersion}}', timeout=15)
    print('Cube Core: pulling pinned ' + CUBE_VERSION + ' image', flush=True)
    command('pull', '--platform', 'linux/amd64', CUBE_IMAGE, timeout=900)
    name = None
    with tempfile.TemporaryDirectory() as directory:
        Path(directory).chmod(0o755)
        file = Path(directory) / 'model.yml'
        file.write_text(model)
        file.chmod(0o644)
        try:
            print('Cube Core: starting', flush=True)
            name, cube = start_cube(command, directory, data_sources)
            presto(cases, session, command,
                   lambda endpoint, cases, session: compare(endpoint, cube, cases, session))
        finally:
            if name:
                print('Cube Core: removing owned container', flush=True)
                command('rm', '--force', name, timeout=30)


def main():
    try:
        pinned()
        # Compile before provisioning, so a compiler failure never starts a container.
        print('Cube Core: compiling execution manifest', flush=True)
        cases, session, model, data_sources = MANIFEST.compiled_sources('presto')
        run(cases, session, model, data_sources)
    except Exception as error:
        print('check conformance: FAILED — ' + failure(error), file=sys.stderr)
        return 1
    print('check conformance: PASSED — Cube Core %s, %d cases: SQL and parameters equal the kernel\'s '
          'but for Cube\'s default row limit, and results equal on Presto %s (%s)'
          % (CUBE_VERSION, len(cases), PRESTO.VERSION.split('-')[0], SCENARIO))
    return 0


if __name__ == '__main__':
    sys.exit(main())
