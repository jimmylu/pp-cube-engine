"""Lifecycle regressions; simulated Docker is not real T2 evidence."""
import contextlib
import io
import importlib.util
from pathlib import Path
import subprocess
import unittest

SPEC = importlib.util.spec_from_file_location(
    'container_runner', Path(__file__).parents[1] / 'engines/presto-container.py')
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class LifecycleTests(unittest.TestCase):
    def exercise(self, fail=None, verify_failure=False, version='"0.293-2f48ff6"'):
        calls, endpoints = [], []

        def command(*args, **kwargs):
            calls.append((args, kwargs))
            if args[0] == fail:
                raise subprocess.CalledProcessError(1, ['docker'])
            if args[0] == 'inspect':
                return '{"8080/tcp":[{"HostPort":"12345"}]}'
            if args[:1] == ('exec',) and 'SELECT DISTINCT node_version FROM system.runtime.nodes' in args:
                return version
            return ''

        def verify(endpoint):
            endpoints.append(endpoint)
            if verify_failure:
                raise ValueError('mismatch')

        if fail or verify_failure or version != '"0.293-2f48ff6"':
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
        for stage in ['create', 'cp', 'start', 'inspect']:
            with self.subTest(stage=stage):
                calls, endpoints = self.exercise(fail=stage)
                self.assertEqual(calls[-1][0][0], 'rm')
                self.assertEqual(endpoints, [])
        calls, _ = self.exercise(verify_failure=True)
        self.assertEqual(calls[-1][0][0], 'rm')

    def test_unexpected_version_fails_before_fixture_and_cleans_up(self):
        calls, endpoints = self.exercise(version='"0.294-other"')
        self.assertEqual(calls[-1][0][0], 'rm')
        self.assertEqual(endpoints, [])
        self.assertFalse(any('input' in kwargs for _, kwargs in calls))

    def test_memory_worker_readiness_precedes_fixture_and_retries_only_no_nodes(self):
        calls = []

        def command(*args, **kwargs):
            calls.append(args)
            if args[0] == 'inspect':
                return '{"8080/tcp":[{"HostPort":"12345"}]}'
            if 'SELECT DISTINCT node_version FROM system.runtime.nodes' in args:
                return '"0.293-2f48ff6"'
            if any('cube_readiness_0 AS SELECT' in arg for arg in args):
                raise subprocess.CalledProcessError(1, 'docker', stderr='No nodes available')
            return ''

        runner.run(command, lambda _: None, lambda _: None)
        probes = [args for args in calls if any(' AS SELECT 1' in arg for arg in args)]
        self.assertEqual(len(probes), 2)
        fixture = next(i for i, args in enumerate(calls) if '--file' in args)
        self.assertLess(calls.index(probes[-1]), fixture)
        self.assertEqual(calls[-1][0], 'rm')

    def test_failure_diagnostics_are_fixed_and_preserve_cleanup(self):
        output = io.StringIO()
        with contextlib.redirect_stderr(output):
            calls, _ = self.exercise(verify_failure=True)
        self.assertIn('engine failure stage: golden-verification', output.getvalue())
        self.assertEqual(calls[-1][0][0], 'rm')
        safe = 'nextUri must remain on the configured origin'
        self.assertEqual(runner.SMOKE.failure_category(ValueError(safe)), safe)
        self.assertEqual(runner.SMOKE.failure_category(ValueError('private endpoint')), 'ValueError')

    def test_create_timeout_attempts_cleanup(self):
        calls = []

        def command(*args, **kwargs):
            calls.append(args)
            if args[0] == 'create':
                raise subprocess.TimeoutExpired('docker', 60)
            return ''

        with self.assertRaises(subprocess.TimeoutExpired):
            runner.run(command)
        self.assertEqual(calls[-1][0], 'rm')

    def test_pre_creation_failure_does_not_remove_unowned_container(self):
        for stage in ['info', 'pull']:
            calls, _ = self.exercise(fail=stage)
            self.assertNotIn('rm', [args[0] for args, _ in calls])


if __name__ == '__main__':
    unittest.main()
