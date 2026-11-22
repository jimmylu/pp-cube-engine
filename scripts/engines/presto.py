#!/usr/bin/env python3
"""Execute one existing parameter-free Presto golden against a prepared fixture."""
import argparse
import json
import os
from pathlib import Path
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SNAPSHOT = ROOT / 'crates/pp-cube-compile/tests/snapshots/aggregation__one_sum_grouped_by_one_dimension.snap'
EXPECTED_COLUMNS = ['sales__channel_cd', 'sales__amount']
EXPECTED_ROWS = [['web', 30], ['store', 7]]


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


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('HTTP redirects are not supported')


def execute(endpoint, sql, timeout=30):
    """A bounded, single-statement client; no session mutation or authentication."""
    origin = urllib.parse.urlsplit(endpoint)
    if (origin.scheme not in ('http', 'https') or not origin.hostname
            or origin.username or origin.password or origin.query or origin.fragment
            or origin.path not in ('', '/')):
        raise ValueError('endpoint must be an HTTP(S) origin without credentials')
    deadline = time.monotonic() + timeout
    opener = urllib.request.build_opener(NoRedirect)
    headers = {'X-Presto-User': 'cube-engine-test', 'X-Presto-Catalog': 'memory',
               'X-Presto-Schema': 'dw', 'X-Presto-Time-Zone': 'UTC',
               'Content-Type': 'text/plain; charset=utf-8'}
    url = endpoint.rstrip('/') + '/v1/statement'
    body = sql.encode('utf-8')
    rows, columns = [], None
    pending = None
    try:
        while True:
            remaining = deadline - time.monotonic()
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
                time.sleep(min(0.05, max(0, deadline - time.monotonic())))
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
                with opener.open(cancel, timeout=1):
                    pass
            except (OSError, ValueError):
                pass


def verify(endpoint, timeout=30):
    columns, rows = execute(endpoint, snapshot_sql(SNAPSHOT.read_text()), timeout)
    if columns != EXPECTED_COLUMNS or rows != EXPECTED_ROWS:
        raise ValueError('result differs from the prepared fixture expectation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint', default=os.environ.get('PRESTO_URL'))
    args = parser.parse_args()
    if not args.endpoint:
        print('check engines: NOT RUN — set PRESTO_URL; see docs/agents/engines.md', file=sys.stderr)
        return 2
    try:
        verify(args.endpoint)
    except (OSError, ValueError, KeyError, TypeError) as error:
        # Do not publish endpoint names, SQL, or server-provided messages.
        print('check engines: FAILED — ' + type(error).__name__, file=sys.stderr)
        return 1
    print('check engines: PASSED — Presto grouped sum golden, 1 case; not full T2 acceptance')
    return 0


if __name__ == '__main__':
    sys.exit(main())
