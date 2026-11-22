"""Lifecycle regressions; simulated Docker is not real T2 evidence."""
import importlib.util
from pathlib import Path
import subprocess
import unittest

SPEC = importlib.util.spec_from_file_location(
    'container_runner', Path(__file__).parents[1] / 'engines/presto-container.py')
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class LifecycleTests(unittest.TestCase):
    def exercise(self, fail=None, verify_failure=False):
        calls, endpoints = [], []

        def command(*args, **kwargs):
            calls.append((args, kwargs))
            if args[0] == fail:
                raise subprocess.CalledProcessError(1, ['docker'])
            if args[0] == 'inspect':
                return '{"8080/tcp":[{"HostPort":"12345"}]}'
            if args[:1] == ('exec',) and 'SELECT version()' in args:
                return '"0.293"'
            return ''

        def verify(endpoint):
            endpoints.append(endpoint)
            if verify_failure:
                raise ValueError('mismatch')

        if fail or verify_failure:
            with self.assertRaises((subprocess.SubprocessError, ValueError, TimeoutError)):
                runner.run(command, verify, lambda _: None)
        else:
            runner.run(command, verify, lambda _: None)
        return calls, endpoints

    def test_success_sets_up_fixture_verifies_and_removes_owned_container(self):
        calls, endpoints = self.exercise()
        self.assertEqual(endpoints, ['http://127.0.0.1:12345'])
        self.assertEqual(calls[0][0][0], 'info')
        self.assertIn(runner.IMAGE, calls[1][0])
        create = next(args for args, _ in calls if args[0] == 'create')
        self.assertIn('127.0.0.1::8080', create)
        name = create[create.index('--name') + 1]
        self.assertEqual(calls[-1][0], ('rm', '--force', name))
        setup = next(kwargs['input'] for _, kwargs in calls if 'input' in kwargs)
        self.assertEqual(setup, (runner.ROOT / 'scripts/engines/presto-fixture.sql').read_text())

    def test_failures_after_creation_always_remove_container(self):
        for stage in ['cp', 'start', 'inspect', 'exec']:
            with self.subTest(stage=stage):
                calls, endpoints = self.exercise(fail=stage)
                self.assertEqual(calls[-1][0][0], 'rm')
                self.assertEqual(endpoints, [])
        calls, _ = self.exercise(verify_failure=True)
        self.assertEqual(calls[-1][0][0], 'rm')

    def test_pre_creation_failure_does_not_remove_unowned_container(self):
        for stage in ['info', 'pull', 'create']:
            calls, _ = self.exercise(fail=stage)
            self.assertNotIn('rm', [args[0] for args, _ in calls])


if __name__ == '__main__':
    unittest.main()
