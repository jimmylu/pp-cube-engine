"""Offline harness regressions; real Docker evidence is a separate tier."""
import datetime
import importlib.util
import json
from pathlib import Path
import subprocess
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

    def test_failure_point_names_stage_case_and_code_but_no_server_text(self):
        class ServerError(Exception):
            errno = 1105
            msg = 'private server message: dw.sales_dtl not found'
        cases = self.manifest()['cases']
        connection = Mock()
        connection.cursor.return_value.execute.side_effect = [None, ServerError('private')]
        with self.assertRaises(ServerError) as failure:
            doris.suite(connection, cases)
        self.assertEqual(doris.failure_point(failure.exception),
                         'stage=fixture case=day statement=2 code=1105 phrases=not found')
        connection = Mock()
        with unittest.mock.patch.object(doris, 'verify', side_effect=[None, ValueError('differs')]):
            with self.assertRaises(ValueError) as failure:
                doris.suite(connection, cases)
        self.assertEqual(doris.failure_point(failure.exception), 'stage=case case=week')
        doris.PROGRESS.update(stage='case', case='x; DROP TABLE y')
        self.assertEqual(doris.failure_point(ServerError('p')), 'stage=case code=1105 phrases=not found')

    def test_storage_readiness_retries_only_the_unregistered_backend_condition(self):
        class ServerError(Exception):
            def __init__(self, errno, msg):
                self.errno, self.msg = errno, msg
        not_ready = ServerError(1105, 'errCode = 2, detailMessage = Failed to find enough backend')
        connection, pauses = Mock(), []
        connection.cursor.return_value.execute.side_effect = [None, not_ready, not_ready, None, None]
        doris.await_storage(connection, pauses.append)
        statements = [call.args[0] for call in connection.cursor.return_value.execute.call_args_list]
        self.assertEqual(pauses, [2, 2])
        self.assertTrue(statements[1].startswith('CREATE TABLE IF NOT EXISTS cube_readiness.probe'))
        self.assertEqual(statements[-1], 'DROP DATABASE cube_readiness FORCE')
        connection.cursor.return_value.close.assert_called_once()
        for error in (ServerError(1105, 'syntax error near value'), ServerError(1064, 'backend'),
                      ValueError('backend')):
            connection = Mock()
            connection.cursor.return_value.execute.side_effect = [None, error]
            with self.subTest(error=error), self.assertRaises(type(error)):
                doris.await_storage(connection, pauses.append)
            self.assertEqual(doris.failure_point(error).split()[0], 'stage=storage-readiness')
        ticks = iter([0, 0, 200])
        connection = Mock()
        connection.cursor.return_value.execute.side_effect = [None, not_ready, not_ready]
        with self.assertRaisesRegex(TimeoutError, 'storage not ready'):
            doris.await_storage(connection, pauses.append, lambda: next(ticks))

    def test_frontend_and_backend_are_pinned_and_share_one_private_network(self):
        calls = []
        owned, frontend = doris.provision(lambda *a, **k: calls.append(a), 'a1b2c3d4e5f6')
        self.assertEqual([c[:2] for c in calls],
                         [('network', 'create'), ('create', '--name'), ('create', '--name'),
                          ('start', frontend), ('start', 'cube-doris-be-a1b2c3d4e5f6')])
        subnet = calls[0][calls[0].index('--subnet') + 1]
        base = subnet[:-len('0/24')]
        self.assertRegex(subnet, r'^172[.](28|29|30|31)[.][0-9]{1,3}[.]0/24$')
        fe, be = calls[1], calls[2]
        # Digests, never tags; the frontend alone is published, on loopback only.
        self.assertEqual((fe[-1], be[-1]), (doris.FE_IMAGE, doris.BE_IMAGE))
        for image in (doris.FE_IMAGE, doris.BE_IMAGE):
            self.assertRegex(image, r'^apache/doris@sha256:[0-9a-f]{64}$')
        self.assertIn('127.0.0.1::9030', fe)
        self.assertNotIn('--publish', be)
        self.assertIn('FE_SERVERS=fe1:' + base + '2:9010', fe)
        self.assertIn('FE_SERVERS=fe1:' + base + '2:9010', be)
        self.assertIn('BE_ADDR=' + base + '3:9050', be)
        self.assertEqual(owned, [('network', 'cube-doris-a1b2c3d4e5f6'), ('container', frontend),
                                 ('container', 'cube-doris-be-a1b2c3d4e5f6')])
        # Released in reverse: containers before the network they are on.
        removed = []
        doris.release(lambda *a, **k: removed.append(a), owned)
        self.assertEqual(removed, [('rm', '--force', 'cube-doris-be-a1b2c3d4e5f6'),
                                   ('rm', '--force', frontend),
                                   ('network', 'rm', 'cube-doris-a1b2c3d4e5f6')])

    def test_a_subnet_in_use_is_skipped_and_a_failed_start_releases_what_exists(self):
        calls = []

        def taken_once(*args, **kwargs):
            calls.append(args)
            if args[:2] == ('network', 'create') and len(calls) == 1:
                raise subprocess.CalledProcessError(1, 'docker')
        owned, _ = doris.provision(taken_once, 'a1b2c3d4e5f6')
        first, second = (c[c.index('--subnet') + 1] for c in calls[:2])
        self.assertNotEqual(first, second)
        self.assertEqual(owned[0], ('network', 'cube-doris-a1b2c3d4e5f6'))

        calls = []

        def backend_fails(*args, **kwargs):
            calls.append(args)
            if args[0] == 'create' and 'cube-doris-be-a1b2c3d4e5f6' in args:
                raise subprocess.CalledProcessError(1, 'docker')
        with self.assertRaises(subprocess.CalledProcessError):
            doris.provision(backend_fails, 'a1b2c3d4e5f6')
        # Nothing this invocation did not create is touched.
        self.assertEqual(calls[-2:], [('rm', '--force', 'cube-doris-fe-a1b2c3d4e5f6'),
                                      ('network', 'rm', 'cube-doris-a1b2c3d4e5f6')])

        def never(*args, **kwargs):
            raise subprocess.CalledProcessError(1, 'docker')
        with self.assertRaises(subprocess.CalledProcessError):
            doris.provision(never, 'a1b2c3d4e5f6', attempts=3)
