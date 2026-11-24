"""Lifecycle, loading and comparison regressions; simulated Docker is not real evidence."""
import importlib.util
from pathlib import Path
import subprocess
import unittest

SPEC = importlib.util.spec_from_file_location(
    'druid_runner', Path(__file__).parents[1] / 'engines/druid-container.py')
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)

SQL = 'SELECT sum(a) "sales__amount" FROM druid.sales_dtl WHERE c = ?'
SESSION = {'useApproximateCountDistinct': 'false'}
STATEMENT = 'REPLACE INTO "sales_dtl" OVERWRITE ALL SELECT 1 PARTITIONED BY ALL'


def case(**changes):
    value = {'name': 'equals', 'scenario': 'B0-DRUID-004', 'fixture': [STATEMENT], 'sql': SQL,
             'params': [{'type': 'text', 'value': 'C1'}], 'columns': ['sales__amount'], 'rows': [[30]],
             'query': {'measures': ['sales.amount']}, 'timezone': 'UTC', 'security_filters': []}
    value.update(changes)
    return value


class LifecycleTests(unittest.TestCase):
    def exercise(self, fail=None, verify_failure=False, model='cubes: []'):
        calls, verified = [], []

        def command(*args, **kwargs):
            calls.append(args)
            if args[0] == fail:
                raise subprocess.CalledProcessError(1, ['docker'])
            if args[0] == 'inspect':
                return '{"8888/tcp":[{"HostPort":"12345"}]}'
            return ''

        def start_cube(command, directory, data_sources, database):
            command('create', '--name', 'cube-core-x')
            self.assertEqual(database['DB_TYPE'], 'druid')
            self.assertEqual((Path(directory) / 'model.yml').read_text(), model)
            return 'cube-core-x', 'http://127.0.0.1:4000'

        def verify(endpoint, cases, session, cube):
            verified.append((endpoint, session, cube))
            if verify_failure:
                raise ValueError('result differs from the case expectation')

        arguments = ([case()], SESSION, model, ['warehouse'], command, verify, lambda endpoint: None,
                     start_cube)
        if fail or verify_failure:
            with self.assertRaises((subprocess.SubprocessError, ValueError)):
                runner.run(*arguments)
        else:
            runner.run(*arguments)
        return calls, verified

    def test_success_verifies_on_the_loopback_port_and_removes_both_containers(self):
        calls, verified = self.exercise()
        self.assertEqual(verified, [('http://127.0.0.1:12345', SESSION, 'http://127.0.0.1:4000')])
        create = next(args for args in calls if args[0] == 'create')
        self.assertIn('127.0.0.1::8888', create)
        self.assertIn(runner.IMAGE, create)
        name = create[create.index('--name') + 1]
        self.assertEqual([args for args in calls if args[0] == 'rm'],
                         [('rm', '--force', 'cube-core-x'), ('rm', '--force', name)])

    def test_the_engine_runs_the_settings_the_cases_need(self):
        script = runner.script()
        self.assertIn('druid.generic.useDefaultValueForNull=false', script)
        self.assertEqual(script.count('druid.processing.numMergeBuffers=6'), 2)
        for service in runner.SERVICES:
            self.assertIn('bin/run-druid ' + service + ' ', script)
        # A background command followed by `;` is a shell syntax error.
        self.assertNotIn('&;', script.replace(' ', ''))
        self.assertTrue(script.endswith('\nwait'))

    def test_failures_after_creation_remove_what_was_created(self):
        for stage in ['start', 'inspect']:
            with self.subTest(stage=stage):
                calls, verified = self.exercise(fail=stage)
                self.assertEqual(calls[-1][0], 'rm')
                self.assertEqual(verified, [])
        calls, _ = self.exercise(verify_failure=True)
        self.assertEqual([args[0] for args in calls[-2:]], ['rm', 'rm'])

    def test_pre_creation_failure_removes_nothing(self):
        for stage in ['info', 'pull']:
            calls, _ = self.exercise(fail=stage)
            self.assertNotIn('rm', [args[0] for args in calls])

    def test_readiness_needs_an_answer_and_the_pinned_version(self):
        answers = iter([OSError('refused'), [{'probe': 1}], {'version': runner.VERSION}])

        def ask(endpoint, path, body=None, timeout=60):
            answer = next(answers)
            if isinstance(answer, Exception):
                raise answer
            return answer

        runner.ready('http://x', ask, lambda _: None)
        with self.assertRaisesRegex(ValueError, 'unexpected engine version'):
            runner.ready('http://x', lambda *a: {'version': '27.0.0'} if a[1] == '/status' else [{'probe': 1}],
                         lambda _: None)
        with self.assertRaisesRegex(TimeoutError, 'Druid did not become ready'):
            runner.ready('http://x', lambda *a: [], lambda _: None, attempts=2)


class LoadTests(unittest.TestCase):
    def load(self, segment_answers, statuses=('RUNNING', 'SUCCESS')):
        segment_answers, statuses, submitted = iter(segment_answers), iter(statuses), []

        def ask(endpoint, path, body=None, timeout=60):
            if path == '/druid/v2/sql/task':
                submitted.append(body['query'])
                return {'taskId': 'query-1'}
            if path.startswith('/druid/indexer/'):
                return {'status': {'status': next(statuses)}}
            self.assertEqual(body['parameters'], [{'type': 'VARCHAR', 'value': 'sales_dtl'}])
            return next(segment_answers)

        runner.load('http://x', STATEMENT, ask, lambda _: None, attempts=3)
        return submitted

    def test_loaded_means_a_new_version_is_served_and_no_older_one(self):
        old, new = ['v1', 1, 0], ['v2', 1, 0]
        # Before; then the old version is still served beside the new one; then only the new.
        self.assertEqual(self.load([[old], [old, new], [['v1', 1, 1], new]]), [STATEMENT])
        with self.assertRaisesRegex(TimeoutError, 'ingested segments were not served'):
            self.load([[old], [old], [old], [old]])

    def test_a_failed_or_endless_task_and_a_foreign_statement_fail(self):
        with self.assertRaisesRegex(ValueError, 'ingestion task failed'):
            self.load([[]], statuses=('FAILED',))
        with self.assertRaisesRegex(TimeoutError, 'ingestion task did not finish'):
            self.load([[]], statuses=('RUNNING', 'RUNNING', 'RUNNING'))
        with self.assertRaisesRegex(ValueError, 'unsupported fixture statement'):
            runner.load('http://x', 'DROP TABLE sales_dtl', lambda *a: self.fail('submitted'))


class SuiteTests(unittest.TestCase):
    def suite(self, cases, answer=(SQL + '\nLIMIT 10000', ['C1']), result=(['sales__amount'], [[30]]),
              cube='http://cube'):
        loaded, executed = [], []

        def run(endpoint, sql, timeout, parameters, session):
            executed.append((sql, parameters, session))
            return result

        runner.suite('http://x', cases, SESSION, cube, run,
                     lambda endpoint, statement: loaded.append(statement), lambda cube, query: answer)
        return loaded, executed

    def test_fixtures_load_once_and_the_kernel_sql_runs_with_its_parameters_and_session(self):
        loaded, executed = self.suite([case(), case(name='again')])
        self.assertEqual(loaded, [STATEMENT])
        self.assertEqual(executed, [(SQL, ({'type': 'text', 'value': 'C1'},), SESSION)] * 2)

    def test_differences_fail_by_category_and_name_the_case(self):
        good = (SQL + '\nLIMIT 10000', ['C1'])
        for answer, result, message, stage in [
                ((SQL, ['C1']), None, 'Cube Core SQL differs from the kernel', 'cube-core-sql'),
                ((good[0], ['C2']), None, 'Cube Core parameters differ from the kernel', 'cube-core-sql'),
                (good, (['sales__amount'], [[31]]), 'result differs from the case expectation', 'execution'),
                (good, (['other'], [[30]]), 'result differs from the case expectation', 'execution')]:
            with self.subTest(message=message):
                with self.assertRaises(ValueError) as failure:
                    self.suite([case()], answer, result)
                self.assertEqual(runner.failure(failure.exception),
                                 message + ' at stage=' + stage + ' case=equals')

    def test_without_cube_core_only_execution_is_checked(self):
        _, executed = self.suite([case()], answer=('other', []), cube=None)
        self.assertEqual(len(executed), 1)

    def test_failures_never_carry_sql_or_server_text(self):
        runner.PROGRESS.update(stage='execution', case='x; DROP TABLE y')
        self.assertEqual(runner.failure(ValueError('SELECT secret FROM druid.sales')),
                         'ValueError at stage=execution')
        self.assertEqual(runner.failure(OSError('http://internal-host:8888')), 'OSError at stage=execution')


class ProtocolTests(unittest.TestCase):
    def test_parameters_travel_apart_and_typed(self):
        bodies = []

        def ask(endpoint, path, body, timeout):
            bodies.append((path, body))
            return [['a'], [1]]

        self.assertEqual(runner.execute('http://x', SQL, 60, (
            {'type': 'text', 'value': "o'k"}, {'type': 'number', 'value': 2},
            {'type': 'bool', 'value': True}), SESSION, ask), (['a'], [[1]]))
        path, body = bodies[0]
        self.assertEqual(path, '/druid/v2/sql')
        self.assertEqual(body['query'], SQL)
        self.assertEqual(body['context'], SESSION)
        self.assertEqual(body['parameters'], [{'type': 'VARCHAR', 'value': "o'k"},
                                              {'type': 'DOUBLE', 'value': 2},
                                              {'type': 'BOOLEAN', 'value': True}])
        for bad in ({'type': 'null'}, {'type': 'text', 'value': 1}, {'type': 'number', 'value': True}):
            with self.assertRaisesRegex(ValueError, 'unsupported manifest parameter'):
                runner.parameter(bad)
        with self.assertRaisesRegex(ValueError, 'invalid query response'):
            runner.execute('http://x', SQL, 60, (), SESSION, lambda *a: {'error': 'x'})

    def test_expected_cells_take_druid_form(self):
        self.assertEqual(runner.cell({'timestamp': '2026-10-01T00:00:00'}), '2026-10-01T00:00:00.000Z')
        self.assertEqual(runner.cell(None), None)
        for bad in ({'timestamp': '2026-10-01'}, [1]):
            with self.assertRaisesRegex(ValueError, 'unsupported expected cell'):
                runner.cell(bad)

    def test_a_manifest_session_may_name_a_druid_context_key(self):
        manifest = {'schema': 1, 'dialect': 'druid', 'session': SESSION,
                    'cases': [{key: case()[key] for key in runner.MANIFEST.CASE_KEYS}]}
        self.assertEqual(runner.MANIFEST.validated(manifest, 'druid', runner.MANIFEST.CASE_KEYS)[1], SESSION)
        manifest['session'] = {'bad key': 'x'}
        with self.assertRaises(ValueError):
            runner.MANIFEST.validated(manifest, 'druid', runner.MANIFEST.CASE_KEYS)


if __name__ == '__main__':
    unittest.main()
