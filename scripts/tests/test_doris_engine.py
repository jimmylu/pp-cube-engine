"""Offline harness regressions; real Docker evidence is a separate tier."""
import datetime
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import Mock

SPEC = importlib.util.spec_from_file_location(
    'doris_engine', Path(__file__).parents[1] / 'engines/doris-container.py')
doris = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(doris)


class DorisTests(unittest.TestCase):
    def test_result_and_column_mismatches_fail(self):
        for columns, rows in [(['wrong'], [(30,)]), (['amount'], [(99,)])]:
            cursor = Mock(column_names=columns)
            cursor.fetchall.return_value = rows
            with self.assertRaisesRegex(ValueError, 'differs'):
                doris.verify(cursor, 'SELECT ?', ('literal',), ['amount'], [(30,)])
            cursor.execute.assert_called_once_with('SELECT ?', ('literal',))

    def manifest(self, **changes):
        case = {'name': 'day', 'scenario': 'B0-DORIS-010', 'fixture': ['CREATE X', 'INSERT X'],
                'sql': 'SELECT ?', 'params': [{'type': 'text', 'value': 'a'}, {'type': 'null'}],
                'columns': ['day', 'amount'], 'rows': [[{'timestamp': '2026-10-01T00:00:00'}, 20]]}
        document = {'schema': 1, 'dialect': 'doris', 'cases': [case, dict(case, name='week')]}
        document.update(changes)
        return document

    def test_manifest_is_compiled_live_and_rejected_unless_complete(self):
        emit = Mock(return_value=Mock(stdout=json.dumps(self.manifest())))
        self.assertEqual([case['name'] for case in doris.compiled_cases(emit)], ['day', 'week'])
        self.assertEqual(emit.call_args.args[0][-4:], ('--bin', 'execution-cases', '--', 'doris'))
        self.assertTrue(emit.call_args.kwargs['check'])
        self.assertEqual(emit.call_args.kwargs['timeout'], 900)
        partial = dict(self.manifest()['cases'][0])
        del partial['rows']
        unloaded = dict(self.manifest()['cases'][0], fixture=[])
        for changes in [{'schema': 2}, {'dialect': 'presto'}, {'cases': []}, {'cases': [partial]},
                        {'cases': [unloaded]}, {'session': {'offset_clause_enabled': 'true'}},
                        {'session': ['x']}, {'session': {'a': 1}}, {'session': {'a b': 'x'}},
                        {'session': {'a': 'x,y=z'}}]:
            emit = Mock(return_value=Mock(stdout=json.dumps(self.manifest(**changes))))
            with self.assertRaises(ValueError):
                doris.compiled_cases(emit)

    def test_manifest_cases_share_fixtures_and_bind_natively(self):
        connection = Mock()
        cases = self.manifest()['cases'] + [dict(self.manifest()['cases'][0], name='dst', fixture=['INSERT Y'])]
        with unittest.mock.patch.object(doris, 'verify') as verify:
            doris.suite(connection, cases)
        statements = [call.args[0] for call in connection.cursor.return_value.execute.call_args_list]
        # An unchanged fixture is loaded once; nothing but case fixtures touches the engine.
        self.assertEqual(statements, ['CREATE X', 'INSERT X', 'INSERT Y'])
        self.assertEqual([call.kwargs for call in connection.cursor.call_args_list],
                         [{}, {'prepared': True}, {'prepared': True}, {}, {'prepared': True}])
        self.assertEqual(verify.call_count, 3)
        self.assertFalse(hasattr(doris, 'golden'))
        self.assertEqual(verify.call_args.args[1:],
                         ('SELECT ?', ('a', None), ['day', 'amount'],
                          [(datetime.datetime(2026, 10, 1), 20)]))

    def test_unknown_manifest_values_fail_closed(self):
        for item in [{'type': 'date', 'value': 'x'}, {'type': 'text', 'value': 1}, {'type': 'text'},
                     {'type': 'number', 'value': True}, {'type': 'null', 'value': 1}]:
            with self.assertRaises(ValueError):
                doris.parameter(item)
        self.assertEqual([doris.parameter({'type': 'number', 'value': 1.5}),
                          doris.parameter({'type': 'bool', 'value': False})], [1.5, False])
        for value in [{'date': '2026-10-01'}, [1]]:
            with self.assertRaises(ValueError):
                doris.cell(value)
        self.assertIsNone(doris.cell(None))
