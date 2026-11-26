"""The kit without a service or an engine: configuration, statements, the model
for an engine, comparison, the report, and the Druid executor against a stand-in."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import tempfile
import threading
import unittest

import grpc

from ppcube import CompiledQuery, CompileRefused
from ppcube.client import Column
from ppcube_itest import cli, config, engines, modelstore, suite, warehouse

REPOSITORY = Path(__file__).resolve().parents[3]
KIT = REPOSITORY / 'tools' / 'integration-kit'


def settings(text):
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'itest.ini'
        path.write_text(text, encoding='utf-8')
        return config.load(str(path))


def compiled(columns, sql='SELECT 1', parameters=(), session=None):
    return CompiledQuery(sql=sql, parameters=tuple(parameters), columns=tuple(Column(*c) for c in columns),
                         session_properties=dict(session or {}), fingerprint='ab' * 32, kernel_version='0.2.0',
                         model_version='itest/x/v1', model_digest='sha256:abc')


SERVICE = '[service]\ntarget = localhost:1\nplaintext = true\n'
TABLE = {'name': 'ins_claim', 'time': 'loss_at',
         'columns': [{'name': 'claim_id', 'type': 'integer'}, {'name': 'note', 'type': 'string'},
                     {'name': 'loss_at', 'type': 'timestamp'}, {'name': 'paid', 'type': 'double'}],
         'rows': [[1, "it's", '2026-01-02T03:04:05', 1.5], [2, None, None, 20]]}


class ConfigTests(unittest.TestCase):
    def test_the_example_configuration_loads_and_holds_no_secret(self):
        loaded = config.load(str(KIT / 'itest.example.ini'))
        self.assertEqual(sorted(loaded.engines), ['doris', 'druid', 'presto'])
        druid = loaded.engines['druid']
        # Druid has one schema; the prefix keeps the test's datasources apart.
        self.assertEqual((druid.schema, druid.table('ins_policy')), ('druid', 'ppcube_itest_ins_policy'))
        self.assertEqual(loaded.engines['presto'].table('ins_policy'), 'ins_policy')
        self.assertEqual(loaded.engines['doris'].option('replication_num'), '3')
        text = (KIT / 'itest.example.ini').read_text()
        self.assertNotRegex(text, r'(?im)^\s*(password|token)\s*=')
        os.environ.pop('DORIS_PASSWORD', None)
        with self.assertRaisesRegex(config.ConfigError, 'DORIS_PASSWORD'):
            loaded.engines['doris'].password()
        with self.assertRaisesRegex(config.ConfigError, 'one of: doris, druid, presto'):
            loaded.engine(None)

    def test_what_cannot_be_tested_safely_is_refused(self):
        engine = '[engine:e]\ndialect = presto\nmodel_version = itest/e/v1\nschema = s\n'
        self.assertEqual(settings(SERVICE + engine).engine(None).name, 'e')
        for text, message in [
                ('[service]\ntarget = h:1\nplaintext = true\ntoken_file = t\n' + engine, 'never sent unencrypted'),
                (SERVICE, 'at least one'),
                (SERVICE + engine.replace('presto', 'trino'), 'presto, doris or druid'),
                (SERVICE + engine.replace('schema = s', 'schema = s; DROP'), 'lower-case'),
                (SERVICE + engine.replace('schema = s\n', ''), 'lower-case'),
                (SERVICE + '[engine:d]\ndialect = druid\nmodel_version = a/v1\ntable_prefix =\n', 'must not be empty'),
                (SERVICE + engine.replace('model_version = itest/e/v1\n', ''), 'model_version')]:
            with self.subTest(message=message):
                with self.assertRaisesRegex(config.ConfigError, message):
                    settings(text)


class WarehouseTests(unittest.TestCase):
    def test_presto_and_doris_statements(self):
        create, insert = warehouse.presto(TABLE, 's', 'ins_claim')
        self.assertEqual(create, 'CREATE TABLE s.ins_claim (claim_id INTEGER, note VARCHAR, loss_at TIMESTAMP, '
                                 'paid DOUBLE)')
        self.assertEqual(insert, "INSERT INTO s.ins_claim VALUES (1,'it''s',TIMESTAMP '2026-01-02 03:04:05',1.5),"
                                 "(2,NULL,NULL,20)")
        create, insert = warehouse.doris(TABLE, 's', 'p_ins_claim', 3)
        self.assertIn("DUPLICATE KEY(claim_id) DISTRIBUTED BY HASH(claim_id) BUCKETS 1 "
                      "PROPERTIES ('replication_num'='3')", create)
        self.assertIn('loss_at DATETIMEV2(3)', create)
        self.assertEqual(insert, "INSERT INTO s.p_ins_claim VALUES (1,'it''s','2026-01-02 03:04:05',1.5),"
                                 "(2,NULL,NULL,20)")

    def test_a_druid_datasource_takes_one_timestamp(self):
        statement = warehouse.druid(TABLE, 'p_ins_claim')
        self.assertTrue(statement.startswith(
            'REPLACE INTO "p_ins_claim" OVERWRITE ALL SELECT "claim_id", "note", TIME_PARSE("loss_at") AS "__time", '
            '"paid" FROM TABLE(EXTERN('))
        # The rows travel as JSON inside a SQL string: a quote in them is doubled.
        self.assertIn('it\'\'s', statement)
        self.assertIn('{"name":"loss_at","type":"string"}', statement)
        self.assertTrue(statement.endswith('PARTITIONED BY ALL'))
        untimed = dict(TABLE, time=None)
        self.assertIn("SELECT TIMESTAMP '2000-01-01 00:00:00' AS \"__time\", \"claim_id\", \"note\", \"loss_at\"",
                      warehouse.druid(untimed, 'x'))

    def test_the_benchmark_tables_are_the_examples(self):
        tables = warehouse.tables(REPOSITORY)
        self.assertEqual(len(tables), 9)
        self.assertTrue(all(table['name'].startswith('ins_') for table in tables))


class ModelTests(unittest.TestCase):
    def engine(self, dialect, **changes):
        text = '[engine:e]\ndialect = %s\nmodel_version = itest/%s/v1\nschema = test_schema\ndata_source = wh\n' % (
            dialect, dialect)
        return settings(SERVICE + text).engine(None)

    def test_the_model_points_at_the_engines_test_tables(self):
        times = {table['name']: table['time'] for table in warehouse.tables(REPOSITORY) if 'time' in table}
        files = modelstore.render(REPOSITORY, self.engine('presto'), times)
        self.assertEqual(len(files), 12)
        policies = files['cubes/policies.yml']
        self.assertIn('sql_table: test_schema.ins_policy\n', policies)
        self.assertIn('data_source: wh\n', policies)
        self.assertIn('{CUBE}.issued_at"', policies)
        self.assertNotIn('dw.', ''.join(files.values()))
        self.assertNotIn('warehouse', ''.join(files[name] for name in files if name.startswith('cubes/')))
        druid = modelstore.render(REPOSITORY, self.engine('druid'), times)
        self.assertIn('sql_table: druid.ppcube_itest_ins_policy\n', druid['cubes/policies.yml'])
        self.assertIn('sql: "{CUBE}.__time"', druid['cubes/policies.yml'])
        self.assertNotIn('issued_at"', druid['cubes/policies.yml'].replace('name: issued_at', ''))
        # A second timestamp of a table stays a column of its own.
        self.assertIn('{CUBE}.reported_at"', druid['cubes/claims.yml'])
        self.assertEqual(files['views/underwriting.yml'], druid['views/underwriting.yml'])

    def test_a_version_is_written_once_in_the_store_layout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = modelstore.write({'manifest.json': '{}', 'cubes/a.yml': 'x'}, Path(directory), 'itest/presto/v1')
            self.assertEqual(root, Path(directory) / 'itest' / 'presto' / 'models' / 'v1')
            self.assertEqual((root / 'cubes' / 'a.yml').read_text(), 'x')
            with self.assertRaises(FileExistsError):
                modelstore.write({'manifest.json': '{}'}, Path(directory), 'itest/presto/v1')
        with self.assertRaises(ValueError):
            modelstore.placed('v1')


class FakeKernel:
    def __init__(self, answer):
        self.answer, self.asked = answer, None

    def compile(self, model_version, dialect, query, **options):
        self.asked = (model_version, dialect, query, options)
        if isinstance(self.answer, Exception):
            raise self.answer
        return self.answer


class FakeExecutor:
    def __init__(self, rows):
        self.rows = rows

    def run(self, compiled):
        if isinstance(self.rows, Exception):
            raise self.rows
        return self.rows


class SuiteTests(unittest.TestCase):
    COLUMNS = [('d', 'cube.day', 'time', 'day'), ('m', 'cube.m', 'number', None), ('s', 'cube.s', 'string', None)]
    EXPECT = {'columns': ['d', 'm', 's'], 'rows': [[{'timestamp': '2026-01-01T00:00:00'}, 1.5, None]]}

    def outcome(self, case, answer=None, rows=None):
        kernel = FakeKernel(answer if answer is not None else compiled(self.COLUMNS))
        result = suite.run(case, kernel, FakeExecutor(rows), 'itest/x/v1', 'presto', 'UTC')
        return result, kernel

    def test_every_status(self):
        good = [['2026-01-01T00:00:00', 1.5, None]]
        case = suite.Case('c', 'title', {'measures': ['cube.m']}, [{'member': 'cube.s'}], self.EXPECT)
        passed, kernel = self.outcome(case, rows=good)
        self.assertEqual((passed.status, passed.note), ('pass', ''))
        # The case's rules travel as security filters, and the request is traceable.
        self.assertEqual(kernel.asked[3]['security_filters'], [{'member': 'cube.s'}])
        self.assertEqual(kernel.asked[3]['request_id'], 'itest/c')
        for rows, note in [([['2026-01-01T00:00:00', 2.5, None]], 'row 1, m: 2.5, expected 1.5'),
                           ([['2026-01-02T00:00:00', 1.5, None]], 'row 1, d:'),
                           ([['2026-01-01T00:00:00', 1.5, '']], "row 1, s: '', expected None"),
                           (good * 2, '2 rows, expected 1'),
                           ([['2026-01-01T00:00:00', 1, None]], 'row 1, m: 1, expected 1.5')]:
            failed, _ = self.outcome(case, rows=rows)
            self.assertEqual(failed.status, 'fail', note)
            self.assertIn(note, failed.note)
        shown, _ = self.outcome(suite.Case('c', 't', {}), rows=good)
        self.assertEqual(shown.status, 'shown')
        broken, _ = self.outcome(case, rows=engines.EngineError('syntax error'))
        self.assertEqual((broken.status, broken.note), ('error', 'the engine refused the SQL: syntax error'))

    def test_a_refusal_is_right_only_when_its_kind_was_expected(self):
        refusal = CompileRefused('unsupported', 'no quarters')
        expected = suite.Case('c', 't', {}, refused={'kind': 'UNSUPPORTED'})
        self.assertEqual(self.outcome(expected, answer=refusal)[0].status, 'refused')
        self.assertEqual(self.outcome(suite.Case('c', 't', {}, refused={'kind': 'QUERY'}), answer=refusal)[0].status,
                         'fail')
        self.assertEqual(self.outcome(suite.Case('c', 't', {}, expect=self.EXPECT), answer=refusal)[0].status, 'fail')
        # Nothing was expected: a person judges it.
        self.assertEqual(self.outcome(suite.Case('c', 't', {}), answer=refusal)[0].status, 'shown')
        got_sql, _ = self.outcome(expected, rows=[])
        self.assertEqual((got_sql.status, got_sql.note), ('fail', 'expected no SQL (UNSUPPORTED), got SQL'))

    def test_the_built_in_suite_is_the_examples_questions_under_each_users_rules(self):
        presto = suite.insurance(REPOSITORY, KIT, 'presto')
        self.assertEqual(len(presto), 17)
        self.assertEqual(len({case.name for case in presto}), 17)
        east = [case for case in presto if case.name.startswith('east.manager/')]
        self.assertTrue(east and all(case.row_filters for case in east))
        self.assertTrue(all(not case.row_filters for case in presto if not case.name.startswith('east.manager/')))
        by_name = {case.name: case for case in presto}
        self.assertEqual(by_name['chief.underwriter/uw_premium_ytd_by_quarter'].refused['kind'], 'UNSUPPORTED')
        for dialect in ('doris', 'druid'):
            self.assertEqual(len(suite.insurance(REPOSITORY, KIT, dialect)), 17)
        self.assertGreater(len(suite.from_file(KIT / 'suites' / 'template.json')), 1)

    def test_the_report_names_what_ran_and_holds_no_secret(self):
        query = compiled(self.COLUMNS, sql='SELECT a FROM t WHERE c = ?', parameters=['East'],
                         session={'offset_clause_enabled': 'true'})
        outcomes = [suite.Outcome(suite.Case('a/one', 'One', {'measures': ['cube.m']}, [{'member': 'cube.s'}]),
                                  'pass', '', query, [['2026-01-01T00:00:00', 1234.5, None]], 3.2, 40.1),
                    suite.Outcome(suite.Case('a/two', 'Two', {}), 'refused', 'no SQL (QUERY): no join path')]
        self.assertEqual(suite.summary(outcomes),
                         '2 cases: 1 passed, 1 refused as expected, 0 shown for comparison by hand, 0 failed, 0 errors')
        with tempfile.TemporaryDirectory() as directory:
            path = suite.report(outcomes, {'Service': 'h:1', 'Engine': 'presto'}, Path(directory))
            text = path.read_text(encoding='utf-8')
            document = json.loads((Path(directory) / 'report.json').read_text(encoding='utf-8'))
        for expected in ['| a/one | PASS | 3 | 40 | 1 |  |', 'SELECT a FROM t WHERE c = ?', 'Parameters: `["East"]`',
                         '"offset_clause_enabled": "true"', 'cube.day', '1,234.5000', '| a/two | REFUSED |']:
            self.assertIn(expected, text)
        self.assertEqual(document['counts']['pass'], 1)
        self.assertEqual(document['cases'][0]['parameters'], ['East'])
        self.assertNotIn('token', text.lower())
        self.assertNotIn('password', text.lower())


class Stub(BaseHTTPRequestHandler):
    seen = []

    properties = {'druid.generic.useDefaultValueForNull': 'false'}

    def do_GET(self):
        if self.path == '/status/properties':
            if Stub.properties is None:
                return self.answer({'error': 'forbidden'}, 403)
            return self.answer(Stub.properties)
        self.answer({'version': '26.0.0'})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        Stub.seen.append((self.path, body, self.headers.get('Authorization')))
        if 'broken' in body['query']:
            return self.answer({'errorMessage': 'Column broken not found'}, 400)
        self.answer([['m', 'd'], [3, '2026-01-01T00:00:00.000Z']])

    def answer(self, document, status=200):
        data = json.dumps(document).encode()
        self.send_response(status)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *arguments):
        pass


class EngineTests(unittest.TestCase):
    def test_values_take_one_form_whatever_the_driver_gives(self):
        import datetime
        import decimal
        for value, kind, expected in [
                ('2026-01-01 00:00:00.000', 'time', '2026-01-01T00:00:00'),
                ('2026-01-01T00:00:00.000Z', 'time', '2026-01-01T00:00:00'),
                (datetime.datetime(2026, 1, 1, 5), 'time', '2026-01-01T05:00:00'),
                (datetime.date(2026, 1, 1), 'time', '2026-01-01T00:00:00'),
                (decimal.Decimal('1.50'), 'number', 1.5), (3, 'number', 3.0), ('7', 'number', 7.0),
                (1, 'boolean', True), (None, 'number', None), ('x', 'string', 'x')]:
            with self.subTest(value=value):
                self.assertEqual(engines.normalized(value, kind), expected)
        query = compiled([('m', 'c.m', 'number', None), ('d', 'c.d', 'time', 'day')])
        # Rows are read by the kernel's column names, in the kernel's order.
        self.assertEqual(engines.arranged(query, ['d', 'm'], [('2026-01-01 00:00:00', 3)]),
                         [[3.0, '2026-01-01T00:00:00']])
        with self.assertRaisesRegex(engines.EngineError, 'no column m'):
            engines.arranged(query, ['d'], [])

    def test_druid_takes_parameters_and_context_apart_from_the_statement(self):
        server = HTTPServer(('127.0.0.1', 0), Stub)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        self.addCleanup(server.shutdown)
        os.environ['KIT_TEST_SECRET'] = 'not-a-real-one'
        engine = settings(SERVICE + '[engine:d]\ndialect = druid\nmodel_version = a/v1\nuser = u\n'
                          'password_env = KIT_TEST_SECRET\nurl = http://127.0.0.1:%d\n' % server.server_port
                          ).engine(None)
        druid = engines.connect(engine)
        key = 'druid.generic.useDefaultValueForNull'
        self.assertEqual(druid.version(), '26.0.0, NULL handling sql-compatible (%s=false)' % key)
        # The mode is the engine's own statement, never a guess from a query.
        for properties, expected in [({key: 'true'}, 'legacy (%s=true)' % key),
                                     ({}, 'not set (the default of this version applies: legacy before Druid 28)'),
                                     (None, 'unknown (the properties of the process cannot be read)')]:
            Stub.properties = properties
            self.assertEqual(druid.null_handling(), expected)
        Stub.properties = {key: 'false'}
        Stub.seen.clear()
        query = compiled([('m', 'c.m', 'number', None), ('d', 'c.d', 'time', 'day')],
                         sql='SELECT m, d FROM druid.t WHERE a = ? AND b > ? AND c = ?',
                         parameters=['East', 2.0, None], session={'useApproximateCountDistinct': 'false'})
        self.assertEqual(druid.run(query), [[3.0, '2026-01-01T00:00:00']])
        path, body, authorization = Stub.seen[0]
        self.assertEqual(path, '/druid/v2/sql')
        self.assertEqual(body['query'], query.sql)
        self.assertEqual(body['parameters'], [{'type': 'VARCHAR', 'value': 'East'}, {'type': 'DOUBLE', 'value': 2.0},
                                              {'type': 'VARCHAR', 'value': None}])
        self.assertEqual(body['context'], {'useApproximateCountDistinct': 'false'})
        self.assertTrue(authorization.startswith('Basic '))
        with self.assertRaisesRegex(engines.EngineError, 'HTTP 400: Column broken not found'):
            druid.sql('SELECT broken')


class CliTests(unittest.TestCase):
    def test_the_commands_and_where_they_run(self):
        commands = cli.parser()._subparsers._group_actions[0].choices
        self.assertEqual(sorted(commands), ['clean', 'compile', 'env', 'load', 'meta', 'model', 'run', 'suite'])
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(cli.main(['--repo', directory, 'env']), 2)
        # The statements of a load can be printed for someone else to run.
        import contextlib
        import io
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'itest.ini'
            path.write_text(SERVICE + '[engine:e]\ndialect = doris\nmodel_version = a/v1\nschema = s\n')
            printed = io.StringIO()
            with contextlib.redirect_stdout(printed):
                self.assertEqual(cli.main(['--repo', str(REPOSITORY), '--config', str(path), 'load', '--dry-run']), 0)
            lines = printed.getvalue().splitlines()
            self.assertEqual(lines[0], 'CREATE DATABASE IF NOT EXISTS s;')
            self.assertEqual(len(lines), 1 + 2 * 9)
            self.assertTrue(all(line.endswith(';') for line in lines))
        # Nothing is dropped without being told to.
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'itest.ini'
            path.write_text(SERVICE + '[engine:e]\ndialect = presto\nmodel_version = a/v1\nschema = s\n')
            self.assertEqual(cli.main(['--repo', str(REPOSITORY), '--config', str(path), 'clean']), 1)


if __name__ == '__main__':
    unittest.main()
