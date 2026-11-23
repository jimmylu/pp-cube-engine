#!/usr/bin/env python3
"""Execute the live-compiled conformance manifest on Presto over its HTTP protocol."""
import argparse
from contextlib import contextmanager
import importlib.util
import http.client
import signal
import json
import subprocess
import math
import os
from pathlib import Path
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('engine_manifest', ROOT / 'scripts/engines/manifest.py')
MANIFEST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MANIFEST)


# Only locally authored, fixed messages may enter public failure evidence.
SAFE_FAILURES = frozenset({
    'unsupported execution manifest', 'malformed execution case', 'unsupported manifest parameter',
    'unsupported expected cell', 'HTTP redirects are not supported',
    'endpoint must be an HTTP(S) origin without credentials', 'invalid query response',
    'nextUri must remain on the configured origin', 'Presto returned a query error',
    'result columns changed between pages', 'unexpected result size',
    'result differs from the case expectation', 'unexpected engine version',
})


def failure_category(error):
    message = str(error)
    return message if type(error) is ValueError and message in SAFE_FAILURES else type(error).__name__


def literal(item):
    """A manifest parameter as a SQL literal for EXECUTE USING; never spliced into the case SQL."""
    kind = item.get('type')
    if kind == 'null' and set(item) == {'type'}:
        return 'NULL'
    value = item.get('value')
    if set(item) != {'type', 'value'}:
        raise ValueError('unsupported manifest parameter')
    if kind == 'text' and isinstance(value, str) and '\x00' not in value:
        return "'" + value.replace("'", "''") + "'"
    if kind == 'bool' and isinstance(value, bool):
        return 'TRUE' if value else 'FALSE'
    if (kind == 'number' and isinstance(value, (int, float)) and not isinstance(value, bool)
            and math.isfinite(value)):
        return 'DOUBLE ' + "'" + repr(float(value)) + "'"
    raise ValueError('unsupported manifest parameter')


def cell(value):
    """An expected cell in the form Presto's JSON protocol returns it."""
    if isinstance(value, dict):
        stamp = value.get('timestamp')
        if list(value) != ['timestamp'] or not isinstance(stamp, str) or len(stamp) != 19 or stamp[10] != 'T':
            raise ValueError('unsupported expected cell')
        return stamp.replace('T', ' ') + '.000'
    if isinstance(value, list):
        raise ValueError('unsupported expected cell')
    return value


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('HTTP redirects are not supported')


@contextmanager
def deadline(seconds):
    """POSIX wall-clock deadline, including slow response-body reads."""
    if seconds <= 0:
        raise TimeoutError('Presto query deadline exceeded')
    started = time.monotonic()
    previous_handler = signal.getsignal(signal.SIGALRM)

    def expired(signum, frame):
        raise TimeoutError('Presto query deadline exceeded')

    signal.signal(signal.SIGALRM, expired)
    previous_timer = signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous_handler)
        if previous_timer[0]:
            signal.setitimer(signal.ITIMER_REAL,
                             max(0.000001, previous_timer[0] - (time.monotonic() - started)),
                             previous_timer[1])


def execute(endpoint, sql, timeout=30, parameters=(), session=None):
    with deadline(timeout):
        return execute_query(endpoint, sql, timeout, parameters, session)


def execute_query(endpoint, sql, timeout, parameters=(), session=None):
    """A bounded, single-statement client; no session mutation or authentication."""
    origin = urllib.parse.urlsplit(endpoint)
    if (origin.scheme not in ('http', 'https') or not origin.hostname
            or origin.username or origin.password or origin.query or origin.fragment
            or origin.path not in ('', '/')):
        raise ValueError('endpoint must be an HTTP(S) origin without credentials')
    cutoff = time.monotonic() + timeout
    opener = urllib.request.build_opener(NoRedirect)
    headers = {'X-Presto-User': 'cube-engine-test', 'X-Presto-Catalog': 'memory',
               'X-Presto-Schema': 'dw', 'X-Presto-Time-Zone': 'UTC',
               'Content-Type': 'text/plain; charset=utf-8'}
    if session:
        # Session properties the manifest declares; the caller of the kernel must set the same.
        headers['X-Presto-Session'] = ','.join(name + '=' + value for name, value in sorted(session.items()))
    url = endpoint.rstrip('/') + '/v1/statement'
    if parameters:
        headers['X-Presto-Prepared-Statement'] = 'cube_case=' + urllib.parse.quote_plus(sql, safe='')
        # EXECUTE USING accepts SQL expressions. The compiled SQL travels unchanged, with its
        # placeholders, as the prepared statement; only the literals are written here.
        sql = 'EXECUTE cube_case USING ' + ', '.join(literal(item) for item in parameters)
    body = sql.encode('utf-8')
    rows, columns = [], None
    pending = None
    try:
        while True:
            remaining = cutoff - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('Presto query deadline exceeded')
            request = urllib.request.Request(url, data=body, headers=headers)
            try:
                with opener.open(request, timeout=remaining) as response:
                    page = json.load(response)
            except urllib.error.HTTPError as error:
                if error.code != 503:
                    raise
                error.close()
                time.sleep(min(0.05, max(0, cutoff - time.monotonic())))
                continue
            if not isinstance(page, dict):
                raise ValueError('invalid query response')
            pending = page.get('nextUri')
            if pending:
                target = urllib.parse.urlsplit(pending)
                if (target.scheme, target.netloc) != (origin.scheme, origin.netloc):
                    pending = None
                    raise ValueError('nextUri must remain on the configured origin')
            if page.get('error'):
                raise ValueError('Presto returned a query error')
            if 'columns' in page:
                names = [column['name'] for column in page['columns']]
                if columns is not None and names != columns:
                    raise ValueError('result columns changed between pages')
                columns = names
            rows.extend(page.get('data', []))
            if len(rows) > 1000:
                raise ValueError('unexpected result size')
            if not pending:
                return columns, rows
            url, body = pending, None
    finally:
        if pending:
            try:
                cancel = urllib.request.Request(pending, headers=headers, method='DELETE')
                with deadline(1), opener.open(cancel, timeout=1):
                    pass
            except (OSError, ValueError, http.client.HTTPException):
                pass


def verify_manifest(endpoint, cases, session=None, timeout=30):
    """Loads each distinct fixture once and compares every case with its literal expectation."""
    loaded = None
    for case in cases:
        if case['fixture'] != loaded:
            for statement in case['fixture']:
                execute(endpoint, statement, timeout, (), session)
            loaded = case['fixture']
        columns, rows = execute(endpoint, case['sql'], timeout, tuple(case['params']), session)
        if columns != case['columns'] or rows != [[cell(value) for value in row] for row in case['rows']]:
            raise ValueError('result differs from the case expectation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint', default=os.environ.get('PRESTO_URL'))
    args = parser.parse_args()
    if not args.endpoint:
        print('check engines: NOT RUN — set PRESTO_URL; see docs/agents/engines.md', file=sys.stderr)
        return 2
    try:
        cases, session = MANIFEST.compiled_manifest('presto')
        verify_manifest(args.endpoint, cases, session)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError,
            http.client.HTTPException) as error:
        # Do not publish endpoint names, SQL, or server-provided messages.
        print('check engines: FAILED — ' + failure_category(error), file=sys.stderr)
        return 1
    print('check engines: PASSED — Presto, %d live-compiled manifest cases (%s)' % (len(cases), MANIFEST.scenarios(cases)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
