"""The back end of an insurer's analytics portal, in Python.

For every user and every question on their dashboard it does what a real back
end does: decide the user's row-level rules, ask the compile service for SQL,
run that SQL on the warehouse with its parameters passed apart and its session
properties applied, and show the rows. The kernel never sees the warehouse, and
the warehouse never sees a Cube query.

It asks what the Java back end beside it asks, and expects the same answers.
Configuration comes from the environment, as in a container; see run.sh.
"""
import json
import os
from pathlib import Path
import sys
import time
from urllib.parse import urlsplit

import grpc
import prestodb

from ppcube import Client, CompileRefused


def env(name):
    value = os.environ.get(name)
    if not value:
        raise SystemExit('Set ' + name + '; run.sh sets it')
    return value


class Portal:
    def __init__(self, kernel, warehouse_url, portal):
        self.kernel = kernel
        # The same address the Java back end gives its driver, without the driver's prefix.
        self.warehouse_url = urlsplit(warehouse_url[len('jdbc:'):] if warehouse_url.startswith('jdbc:') else warehouse_url)
        self.portal = portal
        # Results by fingerprint: the same SQL for the same rules is run once.
        self.results = {}
        self.asked = self.answered = self.refused = self.served = 0
        self.wrong = []

    def warehouse(self, session_properties=None):
        """A connection with what the SQL needs of the engine, as the kernel declared it."""
        catalog, schema = self.warehouse_url.path.strip('/').split('/')
        return prestodb.dbapi.connect(
            host=self.warehouse_url.hostname, port=self.warehouse_url.port, user='analytics-portal',
            catalog=catalog, schema=schema, session_properties=dict(session_properties or {}))

    def await_warehouse(self):
        """A warehouse that was just started answers a moment later than it accepts connections."""
        last = None
        for _ in range(90):
            try:
                cursor = self.warehouse().cursor()
                cursor.execute('CREATE SCHEMA IF NOT EXISTS memory.portal_ready')
                cursor.fetchall()
                cursor.execute('CREATE TABLE IF NOT EXISTS memory.portal_ready.probe AS SELECT 1 AS one')
                cursor.fetchall()
                return
            except Exception as not_yet:  # the driver raises its own and the transport's
                last = not_yet
                time.sleep(2)
        raise SystemExit('The warehouse did not become ready: ' + type(last).__name__)

    def load_warehouse(self, file):
        cursor = self.warehouse().cursor()
        statements = 0
        for line in Path(file).read_text(encoding='utf-8').splitlines():
            sql = line.strip()
            if not sql or sql.startswith('--'):
                continue
            cursor.execute(sql[:-1])
            cursor.fetchall()
            statements += 1
        print('warehouse: loaded, %d statements\n' % statements)

    def describe_model(self):
        """What the portal offers its users to choose from: names and titles, never SQL."""
        version = self.portal['modelVersion']
        # The service reads its store when it starts; a portal that starts beside it waits.
        for attempt in range(61):
            try:
                meta = self.kernel.meta(version)
                break
            except grpc.RpcError as not_yet:
                if attempt == 60 or not_yet.code() not in (grpc.StatusCode.UNAVAILABLE,
                                                             grpc.StatusCode.NOT_FOUND):
                    raise
                time.sleep(1)
        identity = meta.identity
        print('model %s (%s), kernel %s' % (identity.model_version, identity.model_digest[:19],
                                            identity.kernel_version))
        for cube in meta.cubes:
            print('  %-16s %d measures, %d dimensions' % (cube.name, len(cube.measures), len(cube.dimensions)))
        print()

    def dashboard(self, user):
        rules = user['rowFilters']
        print('== %s (%s)' % (user['title'], user['name']))
        print('   row-level rules: %s\n' % (' and '.join(json.dumps(rule) for rule in rules) or 'none'))
        for question in user['questions']:
            self.ask(user['name'], rules, question)

    def ask(self, user, rules, question):
        self.asked += 1
        name = question['name']
        print('-- ' + question['title'])
        try:
            compiled = self.kernel.compile(
                self.portal['modelVersion'], self.portal['dialect'], question['query'],
                timezone=self.portal['timezone'], security_filters=rules, request_id=user + '/' + name)
        except CompileRefused as no_sql:
            # The answer is final: the portal tells the user why, it does not retry.
            print('   no SQL (%s): %s' % (no_sql.kind.upper(), no_sql))
            known = question.get('refused')
            if known and known['kind'] == no_sql.kind.upper():
                print('   %s\n' % known['then'])
                self.refused += 1
            else:
                self.wrong.append('%s/%s: refused, %s' % (user, name, no_sql.kind))
                print()
            return
        if 'refused' in question:
            self.wrong.append('%s/%s: expected a refusal, got SQL' % (user, name))
        cached = compiled.fingerprint in self.results
        if cached:
            self.served += 1
        else:
            self.results[compiled.fingerprint] = self.run(compiled)
        rows = self.results[compiled.fingerprint]
        print('   %d parameters bound, %d session properties, fingerprint %s%s' % (
            len(compiled.parameters), len(compiled.session_properties), compiled.fingerprint[:12],
            ', rows from the cache' if cached else ''))
        show(compiled, rows)
        difference = differs(question['expect'], compiled, rows) if 'expect' in question else None
        if difference:
            self.wrong.append('%s/%s: %s' % (user, name, difference))
            print('   WRONG: ' + difference)
        else:
            self.answered += 1
        print()

    def run(self, compiled):
        """Runs the SQL as it is: parameters handed to the driver, never written into the text."""
        cursor = self.warehouse(compiled.session_properties).cursor()
        cursor.execute(compiled.sql, list(compiled.parameters))
        fetched = cursor.fetchall()
        names = [column[0] for column in cursor.description]
        rows = []
        for record in fetched:
            by_name = dict(zip(names, record))
            row = []
            for column in compiled.columns:
                # The kernel says what each column is; the driver need not be asked.
                value = by_name[column.alias]
                if value is not None and column.type == 'time':
                    value = str(value).replace(' ', 'T').split('.')[0]
                elif value is not None and column.type == 'number':
                    value = float(value)
                row.append(value)
            rows.append(row)
        return rows

    def summary(self):
        print('%d questions asked: %d answered as expected, %d refused as expected, %d served from the cache'
              % (self.asked, self.answered, self.refused, self.served))
        if not self.wrong:
            print('example: PASSED')
            return 0
        for line in self.wrong:
            print('  ' + line)
        print('example: FAILED')
        return 1


def shown(value):
    if value is None:
        return '-'
    if isinstance(value, float):
        return format(int(value), ',d') if value == int(value) else format(value, ',.4f')
    return str(value)


def show(compiled, rows):
    lines = [[column.member for column in compiled.columns]] + [[shown(v) for v in row] for row in rows]
    widths = [max(len(line[index]) for line in lines) for index in range(len(lines[0]))]
    for line in lines:
        print(('   ' + ''.join(cell.ljust(width + 2) for cell, width in zip(line, widths))).rstrip())


def differs(expect, compiled, rows):
    """None when the rows are the expected ones; otherwise what differs."""
    aliases = [column.alias for column in compiled.columns]
    if aliases != expect['columns']:
        return 'columns %s, expected %s' % (aliases, expect['columns'])
    if len(rows) != len(expect['rows']):
        return '%d rows, expected %d' % (len(rows), len(expect['rows']))
    for number, (row, wanted) in enumerate(zip(rows, expect['rows']), 1):
        for alias, got, want in zip(aliases, row, wanted):
            if isinstance(want, dict):
                same = got == want['timestamp']
            elif isinstance(want, bool) or want is None or isinstance(want, str):
                same = got == want
            else:
                same = isinstance(got, float) and abs(got - want) < 1e-9
            if not same:
                return 'row %d, %s: %r, expected %r' % (number, alias, got, want)
    return None


def main():
    portal = json.loads(Path(env('PORTAL_FILE')).read_text(encoding='utf-8'))
    token = Path(env('PPCUBE_TOKEN_FILE')).read_text(encoding='utf-8')
    with Client(env('PPCUBE_TARGET'), trust=env('PPCUBE_CA'), token=token, deadline=20) as kernel:
        back = Portal(kernel, env('WAREHOUSE_URL'), portal)
        back.await_warehouse()
        if os.environ.get('WAREHOUSE_SQL'):
            back.load_warehouse(os.environ['WAREHOUSE_SQL'])
        back.describe_model()
        for user in portal['users']:
            back.dashboard(user)
        # Another session of a user already seen: the SQL is compiled again, cheaply,
        # and the fingerprint finds the rows without the warehouse.
        back.dashboard(portal['users'][0])
        return back.summary()


if __name__ == '__main__':
    sys.exit(main())
