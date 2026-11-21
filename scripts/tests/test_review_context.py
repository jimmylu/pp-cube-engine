"""Review packets use real Git objects; no network or model credentials."""
import importlib.machinery
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

loader = importlib.machinery.SourceFileLoader('review_dev', str(Path(__file__).parents[1] / 'dev'))
spec = importlib.util.spec_from_loader(loader.name, loader)
dev = importlib.util.module_from_spec(spec)
loader.exec_module(dev)


class ReviewContextTests(unittest.TestCase):
    def test_archived_change_and_deleted_base_are_available(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                result = subprocess.check_output(['git', '-C', directory, *args], text=True)
                return result if args[0] == 'show' else result.strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            (root / 'AGENTS.md').write_text('A test may cover multiple scenarios.\n')
            seed = root / 'openspec/changes/seed/specs/determinism/spec.md'
            seed.parent.mkdir(parents=True)
            seed.write_text('Only Repeatable results.\n')
            git('add', '.')
            git('commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD')
            seed.unlink()
            archived = root / 'openspec/changes/archive/2026-11-21-example/design.md'
            archived.parent.mkdir(parents=True)
            archived.write_text('Approved transfer.\n')
            git('add', '.')
            git('commit', '-qm', 'archive')
            bound = dict(base=base, head=git('rev-parse', 'HEAD'), branch='change/example')
            packet = dev.review_packet(bound, git)
            self.assertEqual(packet['files'][f'base:{seed.relative_to(root)}'], 'Only Repeatable results.\n')
            self.assertEqual(packet['files'][f'head:{archived.relative_to(root)}'], 'Approved transfer.\n')
            self.assertIn('base:AGENTS.md', packet['files'])

    def test_findings_require_exact_pinned_evidence(self):
        packet = {'files': {'head:source.rs': 'enum Dialect {\n    Presto,\n}\n'}}
        finding = dict(priority='P1', path='source.rs', line=2, message='Concrete consequence',
                       evidence=[dict(revision='head', path='source.rs', line=2, quote='Presto,')])
        dev.validate_review({'findings': [finding]}, packet, None)
        for field, value in [('quote', 'Doris,'), ('line', 99), ('revision', 'other')]:
            with self.subTest(field=field):
                changed = dict(finding, evidence=[dict(finding['evidence'][0], **{field: value})])
                with self.assertRaisesRegex(ValueError, 'evidence'):
                    dev.validate_review({'findings': [changed]}, packet, None)
        with self.assertRaisesRegex(ValueError, 'evidence'):
            dev.validate_review({'findings': [dict(finding, evidence=[])]}, packet, None)

    def test_previous_findings_cannot_disappear_without_reassessment(self):
        previous = {'findings': [{'message': 'Old claim'}]}
        with self.assertRaisesRegex(ValueError, 'Reassess'):
            dev.validate_review({'findings': []}, {'files': {}}, previous)
        dev.validate_review({'findings': [], 'reassessments': [
            dict(index=0, status='withdrawn', reason='Full base file contradicts old claim')]},
            {'files': {}}, previous)
        with self.assertRaisesRegex(ValueError, 'cannot pass'):
            dev.validate_review({'findings': [], 'reassessments': [
                dict(index=0, status='remaining', reason='Still a defect')]}, {'files': {}}, previous)

    def test_nonancestor_history_does_not_prevent_fresh_review(self):
        from unittest.mock import patch
        import json
        for code in (1, 128):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as directory:
                dev.state = Path(directory)
                dev.repo = 'example/kernel'
                bound = dict(repo=dev.repo, number=1, branch='harness/x', base='a', head='new')
                (dev.state / 'review.json').write_text(json.dumps(dict(bound, head='old', findings=[])))
                with patch.object(dev, 'config', return_value={'reviewer': 'anthropic/opus'}), \
                     patch.object(dev, 'binding', return_value=bound), \
                     patch.object(dev, 'git', side_effect=subprocess.CalledProcessError(code, 'git')), \
                     patch.object(dev, 'review_packet', return_value={'files': {}}), \
                     patch.object(dev, 'model', return_value='{"findings":[]}') as model, \
                     patch.object(dev, 'run', return_value='{"comments":[]}'):
                    report = dev.review()
                    self.assertEqual(report['findings'], [])
                    self.assertIn('"previous_review": null', model.call_args.args[1])

    def test_context_selection_invalidates_cached_questions(self):
        from unittest.mock import patch
        import json
        with tempfile.TemporaryDirectory() as directory:
            dev.state = Path(directory)
            dev.repo = 'example/kernel'
            bound = dict(repo=dev.repo, number=1, branch='harness/x', base='a', head='b')
            old = dict(bound, findings=[], questions=['Need source'], protocol=dev.REVIEW_PROTOCOL,
                       reviewer='anthropic/opus', command=None, context_paths=[])
            (dev.state / 'review.json').write_text(json.dumps(old))
            with patch.object(dev, 'context_paths', ['source.rs']), \
                 patch.object(dev, 'config', return_value={'reviewer': 'anthropic/opus'}), \
                 patch.object(dev, 'binding', return_value=bound), patch.object(dev, 'git'), \
                 patch.object(dev, 'review_packet', return_value={'files': {}}) as packet, \
                 patch.object(dev, 'model', return_value='{"findings":[]}') as model, \
                 patch.object(dev, 'run', return_value='{"comments":[]}'):
                self.assertFalse(dev.review()['questions'])
                model.assert_called_once()
                packet.assert_called_once_with(bound, extra_paths=['source.rs'])
