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

    def test_deliver_resumes_with_uncommitted_repair(self):
        current = {'url': 'https://example.test/1', 'isDraft': True}

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
             patch.object(dev, 'review', return_value={'findings': ['remaining bug']}):
            # Stop after the first review, without invoking a model or GitHub.
            with self.assertRaisesRegex(ValueError, 'budget exhausted'):
                dev.deliver(0)
            git.assert_any_call('commit', '-m', 'fix: resume pending delivery changes')
            self.assertEqual([call.args for call in check.call_args_list], [('fast',), ('ci',), ('engines-container',), ('doris-container',)])

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

    def test_local_engine_failure_blocks_review_and_ready(self):
        modes = []

        def check(mode):
            modes.append(mode)
            if mode == 'engines-container':
                raise subprocess.CalledProcessError(1, ['scripts/check.sh', mode])

        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check', side_effect=check), \
             patch.object(dev, 'review') as review, \
             patch.object(dev, 'run') as runner, \
             patch.object(dev, 'model') as model:
            with self.assertRaises(subprocess.CalledProcessError):
                dev.deliver(2)
            self.assertEqual(modes, ['ci', 'engines-container'])
            review.assert_not_called()
            runner.assert_not_called()
            model.assert_not_called()
            self.assertFalse((dev.state / 'evidence.md').exists())

    def test_doris_failure_blocks_review_and_ready(self):
        def fail_doris(mode):
            if mode == 'doris-container':
                raise subprocess.CalledProcessError(1, ['check', mode])
        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check', side_effect=fail_doris), \
             patch.object(dev, 'review') as review, \
             patch.object(dev, 'run') as runner:
            with self.assertRaises(subprocess.CalledProcessError):
                dev.deliver(0)
            review.assert_not_called()
            runner.assert_not_called()

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
                     patch.object(dev, 'review', return_value={'findings': []}), \
                     patch.object(dev, 'run', return_value=json.dumps(checks)) as runner, \
                     patch.object(dev.time, 'sleep', side_effect=AssertionError('unexpected polling')), \
                     patch.object(dev, 'model') as model:
                    with self.assertRaisesRegex(ValueError, 'Remote t0 failed'):
                        dev.deliver(0)
                    runner.assert_called_once()
                    model.assert_not_called()
                    self.assertFalse((dev.state / 'evidence.md').exists())

    def test_remote_missing_and_pending_checks_wait_for_success(self):
        pending = dict(id=1, name='t0', app={'slug': 'github-actions'},
                       status='in_progress', conclusion=None)
        success = dict(pending, status='completed', conclusion='success')
        responses = iter(json.dumps({'check_runs': rows})
                         for rows in ([], [pending], [success]))

        def run_result(*args, **kwargs):
            if args[:2] == ('gh', 'api'):
                return next(responses)
            return ''

        def review_after_local_checks():
            self.assertEqual([call.args[0] for call in check.call_args_list],
                             ['ci', 'engines-container', 'doris-container'])
            return {'findings': [], 'reviewer': 'other/model'}

        with patch.object(dev, 'publish'), \
             patch.object(dev, 'binding', return_value={'head': 'a', 'number': 1}), \
             patch.object(dev, 'check') as check, \
             patch.object(dev, 'review', side_effect=review_after_local_checks), \
             patch.object(dev, 'config', return_value={'implementer': 'example/model'}), \
             patch.object(dev, 'pr', return_value={'isDraft': True, 'url': 'https://example.test/1'}), \
             patch.object(dev, 'run', side_effect=run_result) as runner, \
             patch.object(dev.time, 'sleep') as sleep:
            dev.deliver(0)
            self.assertEqual(sleep.call_count, 2)
            runner.assert_any_call('gh', 'api',
                                   f'repos/{dev.repo}/commits/a/check-runs?check_name=t0&filter=latest')
            runner.assert_any_call('gh', 'pr', 'ready', '1', '--repo', dev.repo)
            self.assertIn('remote t0 passed', (dev.state / 'evidence.md').read_text())

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

    def test_budget_exhaustion_leaves_pr_draft(self):
        with patch.object(dev, 'publish'), patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check'), patch.object(dev, 'review', return_value={'findings': ['bug']}), \
             patch.object(dev, 'model') as model:
            with self.assertRaisesRegex(ValueError, 'budget exhausted'):
                dev.deliver(0)
            model.assert_not_called()



class ReviewQuestionTests(unittest.TestCase):
    def test_context_questions_block_without_code_repair(self):
        with patch.object(dev, 'publish'), patch.object(dev, 'binding', return_value={'head': 'a'}), \
             patch.object(dev, 'check'), \
             patch.object(dev, 'review', return_value={'findings': [], 'questions': ['Need full type']}), \
             patch.object(dev, 'model') as model:
            with self.assertRaisesRegex(ValueError, 'context clarification'):
                dev.deliver(2)
            model.assert_not_called()


if __name__ == '__main__':
    unittest.main()
