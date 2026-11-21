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
