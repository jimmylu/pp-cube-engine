"""Offline regressions for the Cube Core comparison; real containers are a separate tier."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import Mock

SPEC = importlib.util.spec_from_file_location(
    'cube_core', Path(__file__).parents[1] / 'engines/cube-core.py')
cube = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cube)

SQL = 'SELECT sum(a) "sales__amount"\nFROM t\nWHERE (c = ?)'


def case(**changes):
    value = {'name': 'equals', 'scenario': 'B0-PRESTO-003', 'fixture': ['DROP X', 'CREATE X'],
             'sql': SQL, 'params': [{'type': 'text', 'value': 'C1'}],
             'columns': ['sales__amount'], 'rows': [[30]],
             'query': {'measures': ['sales.amount'],
                       'filters': [{'member': 'sales.c', 'operator': 'equals', 'values': ['C1']}]},
             'timezone': 'Asia/Tokyo', 'security_filters': []}
    value.update(changes)
    return value


class CubeCoreTests(unittest.TestCase):
    def setUp(self):
        cube.PROGRESS.clear()

    def test_the_question_states_timezone_and_appends_security_filters(self):
        secured = case(security_filters=[{'member': 'sales.r', 'operator': 'equals', 'values': ['n']}])
        before = copy.deepcopy(secured)
        query = cube.cube_query(secured)
        self.assertEqual(query['timezone'], 'Asia/Tokyo')
        # Security filters go after the user's, at the top level, and the case is not changed.
        self.assertEqual([f['member'] for f in query['filters']], ['sales.c', 'sales.r'])
        self.assertEqual(secured, before)
        # A timezone the query states wins, as it does in the kernel.
        stated = case(query={'measures': ['sales.amount'], 'timezone': 'UTC'})
        self.assertEqual(cube.cube_query(stated)['timezone'], 'UTC')
        self.assertNotIn('filters', cube.cube_query(stated))

    def test_only_the_default_row_limit_may_differ(self):
        self.assertEqual(cube.expected_sql(case()), SQL + '\nLIMIT 10000')
        limited = case(sql=SQL + '\nLIMIT 5', query={'measures': ['sales.amount'], 'limit': 5})
        self.assertEqual(cube.expected_sql(limited), SQL + '\nLIMIT 5')

    def compare(self, cases, answers, results):
        answers, results = iter(answers), iter(results)
        executed = []

        def execute(endpoint, sql, timeout, parameters, session):
            executed.append((sql, parameters, session))
            return next(results) if parameters or sql.startswith('SELECT') else (None, [])

        cube.compare('http://presto', 'http://cube', cases, {'offset_clause_enabled': 'true'},
                     ask=lambda endpoint, query: next(answers), execute=execute)
        return executed

    def test_equal_sql_and_rows_pass_and_fixtures_load_once(self):
        cases = [case(), case(name='again')]
        answer = (SQL + '\nLIMIT 10000', ['C1'])
        executed = self.compare(cases, [answer, answer],
                                [(['sales__amount'], [[30]]), (['sales__amount'], [[30]])])
        self.assertEqual([entry[0] for entry in executed],
                         ['DROP X', 'CREATE X', answer[0], answer[0]])
        # Cube's own SQL is what runs, with its parameters bound as text and the session set.
        self.assertEqual(executed[2][1], ({'type': 'text', 'value': 'C1'},))
        self.assertEqual(executed[2][2], {'offset_clause_enabled': 'true'})

    def test_any_other_difference_fails_and_names_the_case(self):
        good = (SQL + '\nLIMIT 10000', ['C1'])
        rows = (['sales__amount'], [[30]])
        for answer, result, message, stage in [
                ((SQL, ['C1']), rows, 'Cube Core SQL differs from the kernel', 'cube-core-sql'),
                ((SQL.replace('c = ?', 'c <> ?') + '\nLIMIT 10000', ['C1']), rows,
                 'Cube Core SQL differs from the kernel', 'cube-core-sql'),
                ((good[0], ['C2']), rows, 'Cube Core parameters differ from the kernel', 'cube-core-sql'),
                ((good[0], [1]), rows, 'unsupported Cube Core parameter', 'cube-core-sql'),
                (good, (['sales__amount'], [[31]]), 'Cube Core result differs from the case expectation',
                 'execution'),
                (good, (['other'], [[30]]), 'Cube Core result differs from the case expectation', 'execution')]:
            with self.subTest(message=message, answer=answer[0][-20:]):
                with self.assertRaises(ValueError) as failure:
                    self.compare([case()], [answer], [result])
                self.assertEqual(str(failure.exception), message)
                self.assertEqual(cube.failure(failure.exception),
                                 message + ' at stage=' + stage + ' case=equals')

    def test_failures_never_carry_sql_or_server_text(self):
        cube.PROGRESS.update(stage='execution', case='x; DROP TABLE y')
        self.assertEqual(cube.failure(ValueError('SELECT secret FROM dw.sales')), 'ValueError at stage=execution')
        self.assertEqual(cube.failure(OSError('http://internal-host:4000')), 'OSError at stage=execution')

    def test_the_image_must_be_the_release_of_the_upstream_pin(self):
        listing = lambda commit: Mock(return_value=Mock(stdout='160000 ' + commit + ' 0\tvendor/cube\n'))
        cube.pinned(listing(cube.CUBE_COMMIT))
        for read in (listing('0' * 40), Mock(return_value=Mock(stdout=''))):
            with self.assertRaisesRegex(ValueError, 'does not match the upstream pin'):
                cube.pinned(read)
        self.assertIn('@sha256:', cube.CUBE_IMAGE)

    def test_a_malformed_cube_core_answer_is_refused(self):
        class Response:
            def __init__(self, body):
                self.body = body

            def __enter__(self):
                import io
                return io.StringIO(json.dumps(self.body))

            def __exit__(self, *args):
                pass

        ok = cube.cube_sql('http://cube', {'measures': ['m']}, opener=lambda url, timeout: Response(
            {'sql': {'sql': ['SELECT 1', ['a']]}}))
        self.assertEqual(ok, ('SELECT 1', ['a']))
        for body in ({}, {'error': 'Continue wait'}, {'sql': {'sql': ['SELECT 1']}},
                     {'sql': {'sql': [1, []]}}, {'sql': {'sql': ['SELECT 1', 'a']}}):
            with self.subTest(body=body), self.assertRaisesRegex(ValueError, 'invalid Cube Core response'):
                cube.cube_sql('http://cube', {}, opener=lambda url, timeout: Response(body))

    def test_sources_manifest_is_validated(self):
        def emit(document):
            return Mock(return_value=Mock(stdout=json.dumps(document)))
        document = {'schema': 1, 'dialect': 'presto', 'session': {}, 'cases': [case()],
                    'model': 'cubes: []', 'data_sources': ['warehouse']}
        cases, session, model, sources = cube.MANIFEST.compiled_sources('presto', emit(document))
        self.assertEqual((len(cases), session, model, sources), (1, {}, 'cubes: []', ['warehouse']))
        emitted = emit(document)
        cube.MANIFEST.compiled_sources('presto', emitted)
        self.assertEqual(emitted.call_args.args[0][-2:], ('presto', '--sources'))
        plain = dict(document, cases=[{k: v for k, v in case().items() if k not in cube.MANIFEST.SOURCE_KEYS}])
        for changed in [dict(document, model=''), dict(document, data_sources=[]),
                        dict(document, data_sources=['x; rm']), dict(document, data_sources='warehouse'),
                        dict(document, cases=[case(query='text')]), plain,
                        {k: v for k, v in document.items() if k != 'model'}]:
            with self.assertRaises(ValueError):
                cube.MANIFEST.compiled_sources('presto', emit(changed))


if __name__ == '__main__':
    unittest.main()
