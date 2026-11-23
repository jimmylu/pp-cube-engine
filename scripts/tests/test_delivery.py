"""Offline failure-path tests: no credentials, network or model calls."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest
from unittest.mock import patch

loader = importlib.machinery.SourceFileLoader('delivery', str(Path(__file__).parents[1] / 'dev'))
spec = importlib.util.spec_from_loader(loader.name, loader)
dev = importlib.util.module_from_spec(spec)
loader.exec_module(dev)


class DeliveryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        dev.root = Path(self.tmp.name)
        dev.state = dev.root
        dev.repo = 'example/kernel'
        (dev.root / 'AGENTS.md').write_text('Review only.')
        # Delivery tests leave the slow build checks out unless they say otherwise.
        builds = patch.object(dev, 'builds_expected', return_value=False)
        self.builds_expected = builds.start()
        self.addCleanup(builds.stop)

    def test_ready_transition_triggers_gate(self):
        workflow = (Path(__file__).parents[2] / '.github/workflows/gate.yml').read_text()
        self.assertRegex(workflow, r'pull_request_target:\s+types:\s*\[[^\]]*\bready_for_review\b')

    def test_models_do_not_inherit_github_publication_tokens(self):
        tokens = ('GH_TOKEN', 'GITHUB_TOKEN', 'GH_ENTERPRISE_TOKEN', 'GITHUB_ENTERPRISE_TOKEN')
        command = [sys.executable, '-c',
                   'import json, os; print(json.dumps(dict(os.environ)))']
        cfg = {role + '_command': command for role in ('implementer', 'reviewer')}
        environment = dict.fromkeys(tokens, 'dummy-publication-token')
        environment['MODEL_PROFILE'] = 'dummy-model-profile'
        for role in ('implementer', 'reviewer'):
            with self.subTest(role=role), patch.dict(os.environ, environment, clear=True), \
                 patch.object(dev, 'config', return_value=cfg):
                child_env = json.loads(dev.model(role, 'test prompt'))
                self.assertTrue(set(tokens).isdisjoint(child_env))
                self.assertEqual(child_env['MODEL_PROFILE'], 'dummy-model-profile')
                self.assertTrue(all(os.environ[name] == environment[name] for name in tokens))

    def test_deliver_resumes_pending_changes_and_runs_only_local_ci(self):
        current = {'url': 'https://example.test/1', 'isDraft': True}
        failed = dict(id=1, name='t0', app={'slug': 'github-actions'}, status='completed', conclusion='failure')

        def git_result(*args):
            if args == ('status', '--porcelain'):
                return ' M scripts/dev'
            if args == ('rev-parse', '--short', 'HEAD'):
                return 'abc'
            return ''

        with patch.object(dev, 'config', return_value={}), \
             patch.object(dev, 'identity', return_value=('harness/x', 'abc')), \
             patch.object(dev, 'validate_scope'), patch.object(dev, 'pr', return_value=current), \
             patch.object(dev, 'git', side_effect=git_result) as git, \
             patch.object(dev, 'check') as check, \
             patch.object(dev, 'binding', return_value={'head': 'abc'}), \
             patch.object(dev, 'engines_expected', return_value=False), \
             patch.object(dev, 'run', return_value=json.dumps({'check_runs': [failed]})):
            # Stop at the first remote result, without GitHub.
            with self.assertRaisesRegex(ValueError, 'Remote t0 failed'):
                dev.deliver()
            git.assert_any_call('commit', '-m', 'fix: resume pending delivery changes')
            # No engine container runs locally by default.
            self.assertEqual([call.args for call in check.call_args_list], [('fast',), ('ci',)])

    def test_scope_rejects_product_edit_on_harness_branch(self):
        with patch.object(dev, 'git', side_effect=['crates/model.rs', '']):
            with self.assertRaisesRegex(ValueError, 'travel alone'):
                dev.validate_scope('harness/x')

    def test_scope_rejects_vendor_edits(self):
        with patch.object(dev, 'git', side_effect=['vendor/cube/file', '']):
            with self.assertRaisesRegex(ValueError, 'read-only'):
                dev.validate_scope('change/x')

    def test_status_handles_unpushed_branch(self):
        with patch.object(dev, 'identity', return_value=('harness/x', 'abc')), \
             patch.object(dev, 'git', return_value=''):
            self.assertIsNone(dev.remote_sha())

    def test_local_ci_failure_blocks_remote_wait_and_ready(self):
        def fail(mode):
            raise subprocess.CalledProcessError(1, ['scripts/check.sh', mode])

        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check', side_effect=fail) as check, \
             patch.object(dev, 'run') as runner, \
             patch.object(dev, 'model') as model:
            with self.assertRaises(subprocess.CalledProcessError):
                dev.deliver()
            self.assertEqual([call.args for call in check.call_args_list], [('ci',)])
            runner.assert_not_called()
            model.assert_not_called()
            self.assertFalse((dev.state / 'evidence.md').exists())

    def test_remote_engine_failure_blocks_ready(self):
        def remote(name, conclusion, number):
            return dict(id=number, name=name, app={'slug': 'github-actions'},
                        status='completed', conclusion=conclusion)
        checks = {'check_runs': [remote('t0', 'success', 1), remote('presto-smoke', 'success', 2),
                                 remote('doris-smoke', 'failure', 3)]}
        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check'), \
             patch.object(dev, 'engines_expected', return_value=True), \
             patch.object(dev, 'run', return_value=json.dumps(checks)) as runner:
            with self.assertRaisesRegex(ValueError, 'Remote doris-smoke failed'):
                dev.deliver()
            runner.assert_called_once()
            self.assertFalse((dev.state / 'evidence.md').exists())

    def test_engine_expectation_matches_the_workflow(self):
        workflow = (Path(__file__).parents[2] / '.github/workflows/engines.yml').read_text()
        paths = workflow.split('    paths:\n', 1)[1].split('  workflow_dispatch:', 1)[0]
        self.assertEqual(tuple(line.strip()[3:-1] for line in paths.splitlines() if line.strip()),
                         dev.ENGINE_TRIGGER_PATHS)
        for name in dev.ENGINE_CHECKS:
            self.assertIn('\n  ' + name + ':\n', workflow)
        bound = {'base': 'a', 'head': 'b'}
        for changed, expected in [
                ('crates/pp-cube-dialect/src/lib.rs', True), ('templates/doris.yaml', True),
                ('scripts/engines/doris-container.py', True), ('scripts/check.sh', True),
                ('Cargo.lock', True), ('.github/workflows/engines.yml', True),
                ('docs/agents/delivery.md\0scripts/dev', False),
                ('openspec/specs/dialect-doris/spec.md', False), ('scripts/check.sh.bak', False)]:
            with self.subTest(changed=changed), \
                 patch.object(dev, 'git', return_value=changed + '\0') as git:
                self.assertIs(dev.engines_expected(bound), expected)
                git.assert_called_once_with('diff', '--no-renames', '--name-only', '-z', 'a...b')

    def test_build_expectation_matches_the_workflow(self):
        workflow = (Path(__file__).parents[2] / '.github/workflows/build.yml').read_text()
        paths = workflow.split('    paths:\n', 1)[1].split('  push:', 1)[0]
        self.assertEqual(tuple(line.strip()[3:-1] for line in paths.splitlines() if line.strip()),
                         dev.BUILD_TRIGGER_PATHS)
        for name in dev.BUILD_CHECKS:
            self.assertIn('\n  ' + name + ':\n', workflow)
        # Merges are covered too, because a change to the crates alone does not trigger it.
        self.assertRegex(workflow, r'push:\s+branches: \[main\]')
        for changed, expected in [
                ('Dockerfile', True), ('.dockerignore', True), ('Cargo.lock', True),
                ('scripts/image-check.sh', True), ('.github/workflows/build.yml', True),
                ('crates/pp-cube-service/src/main.rs', False), ('docs/guide.md', False),
                ('scripts/dev', False)]:
            with self.subTest(changed=changed), \
                 patch.object(dev, 'git', return_value=changed + '\0'):
                self.assertIs(dev.triggered({'base': 'a', 'head': 'b'}, dev.BUILD_TRIGGER_PATHS), expected)

    def test_triggered_build_checks_are_required_and_reported(self):
        self.builds_expected.return_value = True

        def remote(name, conclusion, number):
            return dict(id=number, name=name, app={'slug': 'github-actions'},
                        status='completed', conclusion=conclusion)
        failing = {'check_runs': [remote('t0', 'success', 1), remote('image', 'success', 2),
                                  remote('aarch64', 'failure', 3)]}
        with patch.object(dev, 'publish'), patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check'), patch.object(dev, 'engines_expected', return_value=False), \
             patch.object(dev, 'run', return_value=json.dumps(failing)):
            with self.assertRaisesRegex(ValueError, 'Remote aarch64 failed'):
                dev.deliver()
            self.assertFalse((dev.state / 'evidence.md').exists())
        passing = {'check_runs': [remote('t0', 'success', 1), remote('image', 'success', 2),
                                  remote('aarch64', 'success', 3)]}
        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a', 'number': 1}), \
             patch.object(dev, 'check'), patch.object(dev, 'engines_expected', return_value=False), \
             patch.object(dev, 'config', return_value={'implementer': 'example/model'}), \
             patch.object(dev, 'pr', return_value={'isDraft': False, 'url': 'https://example.test/1'}), \
             patch.object(dev, 'run', return_value=json.dumps(passing)):
            dev.deliver()
        self.assertIn('Remote build checks passed: image, aarch64', (dev.state / 'evidence.md').read_text())

    def test_idempotent_network_calls_are_retried_and_others_are_not(self):
        done = subprocess.CompletedProcess([], 0, stdout='ok\n')
        failure = subprocess.CalledProcessError(1, ['gh'])
        with patch.object(dev.subprocess, 'run', side_effect=[failure, failure, done]) as process, \
             patch.object(dev.time, 'sleep') as sleep:
            self.assertEqual(dev.run('gh', 'api', 'repos/x'), 'ok')
            self.assertEqual((process.call_count, sleep.call_count), (3, 2))
        for command in (('gh', 'pr', 'comment', '1'), ('gh', 'pr', 'create'), ('gh', 'pr', 'ready', '1'),
                        ('git', 'commit', '-m', 'x'), ('scripts/check.sh', 'ci')):
            with self.subTest(command=command), \
                 patch.object(dev.subprocess, 'run', side_effect=failure) as process, \
                 patch.object(dev.time, 'sleep') as sleep:
                with self.assertRaises(subprocess.CalledProcessError):
                    dev.run(*command)
                self.assertEqual((process.call_count, sleep.call_count), (1, 0))

    def test_pending_snapshot_blocks_checkpoint(self):
        with patch.object(dev, 'git', side_effect=['crates/a.snap.new', '']):
            with self.assertRaisesRegex(ValueError, 'Pending snapshots'):
                dev.validate_scope('change/x')

    def test_latest_ci_success_supersedes_failed_attempt(self):
        old = dict(id=1, name='t0', app={'slug': 'github-actions'}, status='completed', conclusion='failure')
        latest = dict(old, id=2, conclusion='success')
        self.assertEqual(dev.latest_t0([latest, old]), ['SUCCESS'])

    def test_latest_pending_attempt_does_not_reuse_old_success(self):
        old = dict(id=1, name='t0', app={'slug': 'github-actions'}, status='completed', conclusion='success')
        latest = dict(old, id=2, status='in_progress', conclusion=None)
        self.assertEqual(dev.latest_t0([old, latest]), ['PENDING'])

    def test_remote_non_success_conclusions_block_without_polling(self):
        for conclusion in ('failure', 'cancelled', 'timed_out', 'error', 'skipped',
                           'neutral', 'action_required', 'stale', 'startup_failure'):
            with self.subTest(conclusion=conclusion):
                checks = {'check_runs': [
                    dict(id=1, name='t0', app={'slug': 'github-actions'},
                         status='completed', conclusion=conclusion)]}
                with patch.object(dev, 'publish'), \
                     patch.object(dev, 'binding', return_value={'head': 'a'}), \
                     patch.object(dev, 'check'), \
                     patch.object(dev, 'engines_expected', return_value=False), \
                     patch.object(dev, 'run', return_value=json.dumps(checks)) as runner, \
                     patch.object(dev.time, 'sleep', side_effect=AssertionError('unexpected polling')), \
                     patch.object(dev, 'model') as model:
                    with self.assertRaisesRegex(ValueError, 'Remote t0 failed'):
                        dev.deliver()
                    runner.assert_called_once()
                    model.assert_not_called()
                    self.assertFalse((dev.state / 'evidence.md').exists())

    def test_remote_missing_and_pending_checks_wait_for_success(self):
        def remote(name, number, done=True):
            return dict(id=number, name=name, app={'slug': 'github-actions'},
                        status='completed' if done else 'in_progress',
                        conclusion='success' if done else None)
        t0, presto, doris = remote('t0', 1), remote('presto-smoke', 2), remote('doris-smoke', 3)
        responses = iter(json.dumps({'check_runs': rows}) for rows in (
            [], [remote('t0', 1, done=False)], [t0], [t0, presto, remote('doris-smoke', 3, done=False)],
            [t0, presto, doris]))

        def run_result(*args, **kwargs):
            if args[:2] == ('gh', 'api'):
                return next(responses)
            return ''

        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a', 'number': 1}), \
             patch.object(dev, 'check') as check, \
             patch.object(dev, 'engines_expected', return_value=True), \
             patch.object(dev, 'review', side_effect=AssertionError('review is on demand')), \
             patch.object(dev, 'model', side_effect=AssertionError('delivery runs no model')), \
             patch.object(dev, 'config', return_value={'implementer': 'example/model'}), \
             patch.object(dev, 'pr', return_value={'isDraft': True, 'url': 'https://example.test/1'}), \
             patch.object(dev, 'run', side_effect=run_result) as runner, \
             patch.object(dev.time, 'sleep') as sleep:
            dev.deliver()
            # Engine checks that have not appeared yet are waited for, not assumed.
            self.assertEqual(sleep.call_count, 4)
            self.assertEqual([call.args for call in check.call_args_list], [('ci',)])
            runner.assert_any_call('gh', 'api',
                                   f'repos/{dev.repo}/commits/a/check-runs?filter=latest&per_page=100')
            runner.assert_any_call('gh', 'pr', 'ready', '1', '--repo', dev.repo)
            evidence = (dev.state / 'evidence.md').read_text()
            self.assertIn('Local CI passed; remote t0 passed', evidence)
            self.assertIn('Remote engine containers passed: presto-smoke, doris-smoke', evidence)
            self.assertIn('Local engine containers are on demand and were not run', evidence)
            self.assertIn('Cross-model review is on demand and was not run at this head', evidence)

    def test_evidence_reports_untriggered_engines_and_an_on_demand_review(self):
        t0 = dict(id=1, name='t0', app={'slug': 'github-actions'}, status='completed', conclusion='success')
        dev.save(dev.state / 'review.json', {'head': 'a', 'reviewer': 'other/model',
                                             'findings': [], 'questions': ['q']})
        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a', 'number': 1}), \
             patch.object(dev, 'check'), patch.object(dev, 'engines_expected', return_value=False), \
             patch.object(dev, 'config', return_value={'implementer': 'example/model'}), \
             patch.object(dev, 'pr', return_value={'isDraft': False, 'url': 'https://example.test/1'}), \
             patch.object(dev, 'run', return_value=json.dumps({'check_runs': [t0]})):
            dev.deliver()
        evidence = (dev.state / 'evidence.md').read_text()
        self.assertIn('Remote engine containers were not triggered', evidence)
        self.assertIn('Remote build checks were not triggered', evidence)
        self.assertIn('Cross-model review on demand at this head by other/model: 0 findings, 1 questions', evidence)

    def test_atomic_state_roundtrip(self):
        p = dev.state / 'evidence.json'
        dev.save(p, {'head': 'a'})
        dev.save(p, {'head': 'b'})
        self.assertEqual(json.loads(p.read_text()), {'head': 'b'})
        self.assertFalse(p.with_suffix('.tmp').exists())

    def test_missing_configuration_is_actionable(self):
        with self.assertRaisesRegex(ValueError, 'configure'):
            dev.config()

    def test_rejects_unapproved_branch(self):
        with patch.object(dev, 'git', return_value='main'):
            with self.assertRaisesRegex(ValueError, 'change/'):
                dev.identity()

    def test_dirty_review_rejected(self):
        with patch.object(dev, 'git', return_value=' M file'):
            with self.assertRaisesRegex(ValueError, 'clean'):
                dev.binding()

    def test_pr_mismatch_rejected(self):
        with patch.object(dev, 'clean'), patch.object(dev, 'identity', return_value=('harness/x', 'abc')), \
             patch.object(dev, 'pr', return_value={'headRefOid': 'def'}):
            with self.assertRaisesRegex(ValueError, 'head must match'):
                dev.binding()

    def test_resume_reuses_existing_pr(self):
        current = {'url': 'https://example.test/1'}
        with patch.object(dev, 'config', return_value={}), patch.object(dev, 'identity', return_value=('harness/x', 'abc')), \
             patch.object(dev, 'git', return_value=''), patch.object(dev, 'pr', return_value=current), \
             patch.object(dev, 'run') as runner:
            self.assertEqual(dev.publish(), current)
            runner.assert_not_called()

    def test_stale_review_is_not_saved(self):
        a = {'head': 'a', 'base': 'b', 'branch': 'harness/x'}
        b = dict(a, head='c')
        with patch.object(dev, 'config', return_value={'reviewer': 'other/model'}), \
             patch.object(dev, 'binding', side_effect=[a, b]), patch.object(dev, 'git', return_value='diff'), \
             patch.object(dev, 'model', return_value='{"findings":[]}'):
            with self.assertRaisesRegex(ValueError, 'stale'):
                dev.review()
            self.assertFalse((dev.state / 'review.json').exists())

    def test_malformed_model_reply_is_not_pass(self):
        with patch.object(dev, 'config', return_value={}), \
             patch.object(dev, 'binding', return_value={'base': 'a', 'head': 'b', 'branch': 'harness/x'}), \
             patch.object(dev, 'git', return_value='diff'), patch.object(dev, 'model', return_value='{}'):
            with self.assertRaisesRegex(ValueError, 'findings'):
                dev.review()

    def test_non_object_model_reply_is_controlled_and_not_saved(self):
        for report in ([], ['finding'], 'text', None, True, 42):
            with self.subTest(report=report), \
                 patch.object(dev, 'config', return_value={}), \
                 patch.object(dev, 'binding', return_value={'base': 'a', 'head': 'b', 'branch': 'harness/x'}), \
                 patch.object(dev, 'git', return_value='diff'), \
                 patch.object(dev, 'model', return_value=json.dumps(report)), \
                 patch.object(dev, 'run') as runner:
                with self.assertRaisesRegex(ValueError, 'Reviewer'):
                    dev.review()
                runner.assert_not_called()
                self.assertFalse((dev.state / 'review.json').exists())
                self.assertFalse((dev.state / 'review.md').exists())


if __name__ == '__main__':
    unittest.main()
