"""Offline harness regressions; real Docker evidence is a separate tier."""
import importlib.util
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

    def test_suite_checks_fixture_outcomes_and_uses_prepared_cursor(self):
        connection = Mock()
        with unittest.mock.patch.object(doris, 'verify') as verify:
            doris.suite(connection)
        connection.cursor.assert_any_call(prepared=True)
        self.assertEqual(verify.call_count, 19)
        # Security AND the complete user OR produces two allowed fixture rows.
        self.assertEqual(verify.call_args.args[-1], [(18.0,)])
