#!/usr/bin/env python3
"""Execute existing aggregate and text-filter Presto goldens against a prepared fixture."""
import argparse
from contextlib import contextmanager
import http.client
import signal
import json
import os
import re
from pathlib import Path
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SNAPSHOT = ROOT / 'crates/pp-cube-compile/tests/snapshots/aggregation__one_sum_grouped_by_one_dimension.snap'
EXPECTED_COLUMNS = ['sales__channel_cd', 'sales__amount']
EXPECTED_ROWS = [['C1', 30], ['C2', 7]]
FILTER_SNAPSHOT = ROOT / 'crates/pp-cube-compile/tests/snapshots/filters__equals_on_a_string_dimension.snap'


# Only locally authored, fixed messages may enter public failure evidence.
SAFE_FAILURES = frozenset({
    'invalid golden header', 'only explicitly empty parameters are supported',
    'expected a SELECT golden', 'HTTP redirects are not supported',
    'endpoint must be an HTTP(S) origin without credentials', 'invalid query response',
    'nextUri must remain on the configured origin', 'Presto returned a query error',
    'result columns changed between pages', 'unexpected result size',
    'result differs from the prepared fixture expectation', 'unexpected engine version',
})


def failure_category(error):
    message = str(error)
    return message if type(error) is ValueError and message in SAFE_FAILURES else type(error).__name__


def snapshot_sql(text):
    sections = text.split('---\n', 2)
    if len(sections) != 3 or sections[0]:
        raise ValueError('invalid golden header')
    sql, separator, parameters = sections[2].rpartition('\n-- parameters: ')
    if not separator or parameters.strip() != '[]':
        raise ValueError('only explicitly empty parameters are supported')
    if not sql.lstrip().startswith('SELECT '):
        raise ValueError('expected a SELECT golden')
    return sql


def text_filter_snapshot(text):
    """Read the one supported snapshot shape; Rust Debug is not a general wire format."""
    body, separator, parameters = text.rpartition('\n-- parameters: ')
    match = re.fullmatch(r'\[Text\(("(?:[^"\\]|\\.)*")\)\]\s*', parameters)
    if not separator or not match:
        raise ValueError('expected one text snapshot parameter')
    value = json.loads(match.group(1))
    return snapshot_sql(body + '\n-- parameters: []'), (value,)


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


def execute(endpoint, sql, timeout=30, parameters=()):
    with deadline(timeout):
        return execute_query(endpoint, sql, timeout, parameters)


def execute_query(endpoint, sql, timeout, parameters=()):
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
    url = endpoint.rstrip('/') + '/v1/statement'
    if parameters:
        if len(parameters) != 1 or not isinstance(parameters[0], str) or '\x00' in parameters[0]:
            raise ValueError('expected one text snapshot parameter')
        headers['X-Presto-Prepared-Statement'] = 'cube_smoke=' + urllib.parse.quote_plus(sql, safe='')
        # EXECUTE USING accepts SQL expressions. Quote a text literal, retaining
        # the original golden and its placeholder in the prepared statement.
        sql = "EXECUTE cube_smoke USING '" + parameters[0].replace("'", "''") + "'"
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


def verify(endpoint, timeout=30):
    columns, rows = execute(endpoint, snapshot_sql(SNAPSHOT.read_text()), timeout)
    if columns != EXPECTED_COLUMNS or rows != EXPECTED_ROWS:
        raise ValueError('result differs from the prepared fixture expectation')


def verify_suite(endpoint, timeout=30):
    verify(endpoint, timeout)
    sql, parameters = text_filter_snapshot(FILTER_SNAPSHOT.read_text())
    columns, rows = execute(endpoint, sql, timeout, parameters)
    if columns != EXPECTED_COLUMNS or rows != [['C1', 30]]:
        raise ValueError('result differs from the prepared fixture expectation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint', default=os.environ.get('PRESTO_URL'))
    args = parser.parse_args()
    if not args.endpoint:
        print('check engines: NOT RUN — set PRESTO_URL; see docs/agents/engines.md', file=sys.stderr)
        return 2
    try:
        verify_suite(args.endpoint)
    except (OSError, ValueError, KeyError, TypeError, http.client.HTTPException) as error:
        # Do not publish endpoint names, SQL, or server-provided messages.
        print('check engines: FAILED — ' + failure_category(error), file=sys.stderr)
        return 1
    print('check engines: PASSED — Presto grouped sum and text binding goldens, 2 cases; not full T2 acceptance')
    return 0


if __name__ == '__main__':
    sys.exit(main())
