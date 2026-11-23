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
    def test_compact_transport_preserves_lf_lines_and_citations(self):
        import json
        sources = ['', '\n', 'one\n\n', 'CR\rLF\r\nUnicode\u2028inside\nlast', '1:prefix\nno final LF']
        for source in sources:
            with self.subTest(source=source):
                packet = {'files': {'head:source.rs': source}, 'binary_files': {},
                          'previous_review': {'findings': []}}
                transported = json.loads(dev.encode_review_packet(packet))
                numbered = transported['files']['head:source.rs']
                restored = '\n'.join(line.split(':', 1)[1] for line in dev.source_lines(numbered))
                if numbered.endswith('\n'):
                    restored += '\n'
                self.assertEqual(restored, source)
                self.assertEqual(transported['previous_review'], packet['previous_review'])
                for index, text in enumerate(dev.source_lines(source), 1):
                    if text.strip():
                        dev.validate_review({'findings': [dict(priority='P2', path='source.rs', line=index,
                            message='Test consequence', evidence=[dict(revision='head', path='source.rs',
                            line=index, quote=text)])]}, packet, None)

    def test_dependency_sized_packet_fits_without_omitting_lockfile(self):
        import json
        lock = ''.join(f'name = "package_{n}"\nversion = "1.0.0"\n\n' for n in range(2200))
        packet = {'files': {'base:Cargo.lock': lock, 'head:Cargo.lock': lock + '# change\n'},
                  'binary_files': {}, 'diff': 'dependency change',
                  'previous_review': {'findings': []}}
        old = dict(packet, files={p: [{'line': n, 'text': t} for n, t in enumerate(dev.source_lines(s), 1)]
                                  for p, s in packet['files'].items()})
        self.assertGreater(len(json.dumps(old).encode()), 500_000)
        # Exercise the actual base/head Git extraction for a dependency-only diff.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                raw = subprocess.check_output(['git', '-C', directory, *args])
                return raw if args[0] == 'show' else raw.decode().strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            (root / 'Cargo.lock').write_text(lock)
            git('add', '.')
            git('commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD')
            (root / 'Cargo.lock').write_text(lock + '# change\n')
            git('add', '.')
            git('commit', '-qm', 'dependency')
            pinned = dev.review_packet(dict(base=base, head=git('rev-parse', 'HEAD'),
                                           branch='change/dependency'), git)
            self.assertEqual(pinned['files'], packet['files'])
            self.assertLess(len(dev.encode_review_packet(pinned).encode()), dev.REVIEW_PACKET_LIMIT)
        encoded = dev.encode_review_packet(packet)
        self.assertLess(len(encoded.encode()), dev.REVIEW_PACKET_LIMIT)
        self.assertEqual(set(json.loads(encoded)['files']), set(packet['files']))
        self.assertEqual(dev.REVIEW_PACKET_LIMIT, 750_000)
        # A lockfile edit beside 300 KB of other evidence exceeded the earlier 500 KB cap.
        roomy = dict(packet, files=dict(packet['files'], **{'head:evidence': ('x' * 99 + '\n') * 3000}))
        size = len(dev.encode_review_packet(roomy).encode())
        self.assertTrue(500_000 < size < dev.REVIEW_PACKET_LIMIT, size)
        with self.assertRaisesRegex(ValueError, '750 KB'):
            dev.encode_review_packet({'files': {'head:large': 'x' * 750_000}})

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
                    self.assertIn('"previous_review":null', model.call_args.args[1])

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

    def test_diverged_main_uses_diff_preimage_not_main_tip(self):
        with tempfile.TemporaryDirectory() as directory:
            from unittest.mock import patch
            def git(*args):
                output = subprocess.check_output(['git', '-C', directory, *args])
                return output if args[0] == 'show' else output.decode().strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            source = Path(directory) / 'source.txt'
            source.write_text('original\n')
            git('add', '.')
            git('commit', '-qm', 'common')
            common = git('rev-parse', 'HEAD')
            source.write_text('branch change\n')
            git('commit', '-qam', 'branch')
            head = git('rev-parse', 'HEAD')
            git('checkout', '-q', '--detach', common)
            source.write_text('main only\n')
            git('commit', '-qam', 'main advanced')
            bound = dict(base=git('rev-parse', 'HEAD'), head=head, branch='harness/x')
            packet = dev.review_packet(bound, git)
            self.assertEqual(packet['revisions']['base'], common)
            self.assertEqual(packet['files']['base:source.txt'], 'original\n')
            self.assertNotIn('main only', packet['diff'])

    def test_binary_and_non_utf8_content_is_preserved(self):
        import base64
        with tempfile.TemporaryDirectory() as directory:
            def git(*args):
                output = subprocess.check_output(['git', '-C', directory, *args])
                return output if args[0] == 'show' else output.decode(errors='backslashreplace').strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            (Path(directory) / 'normal.txt').write_text('base\n')
            git('add', '.')
            git('commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD')
            blobs = {'image.bin': b'\x00\xff\x80', 'legacy.txt': b'text\xe9\n'}
            for name, value in blobs.items():
                (Path(directory) / name).write_bytes(value)
            git('add', '.')
            git('commit', '-qm', 'nontext')
            import os
            bound = dict(base=base, head=git('rev-parse', 'HEAD'), branch='harness/x')
            previous_cwd = os.getcwd()
            try:
                os.chdir(directory)
                packet = dev.review_packet(bound)
            finally:
                os.chdir(previous_cwd)
            for name, value in blobs.items():
                encoded = packet['binary_files']['head:' + name]
                self.assertEqual(base64.b64decode(encoded['base64']), value)
                self.assertEqual(encoded['size'], len(value))
                self.assertNotIn('head:' + name, packet['files'])

    def test_citations_follow_git_lf_lines_not_unicode_separators(self):
        source = 'first\fstill first\u2028same line\r\nsecond\x85same second\nlast'
        packet = {'files': {'head:source.txt': source}}
        finding = dict(priority='P3', path='source.txt', line=2, message='Concrete defect',
                       evidence=[dict(revision='head', path='source.txt', line=2, quote='second')])
        dev.validate_review({'findings': [finding]}, packet, None)
        self.assertEqual(dev.source_lines(source), [
            'first\fstill first\u2028same line\r', 'second\x85same second', 'last'])
        self.assertEqual(dev.source_lines('one\n'), ['one'])
        self.assertEqual(dev.source_lines(''), [])

    def test_quoted_and_multiline_paths_are_read_without_git_quoting(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            def git(*args):
                return subprocess.check_output(['git', '-C', directory, *args], text=True).strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            (Path(directory) / 'normal.txt').write_text('base\n')
            git('add', '.')
            git('commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD')
            names = ['中文.md', 'a"b.txt', 'line\nbreak.txt', ' leading space.txt']
            for name in names:
                (Path(directory) / name).write_text('content\n')
            git('add', '.')
            git('commit', '-qm', 'paths')
            parent = git('rev-parse', 'HEAD')
            # Build the raw path in Git objects: some filesystems reject such names.
            blob = subprocess.check_output(['git', '-C', directory, 'hash-object', '-w', '--stdin'],
                                           input=b'content\n').strip()
            entries = subprocess.check_output(['git', '-C', directory, 'ls-tree', '-z', parent])
            entries += b'100644 blob ' + blob + b'\traw\xff.txt\0'
            tree = subprocess.check_output(['git', '-C', directory, 'mktree', '-z'], input=entries).strip()
            head = subprocess.check_output(['git', '-C', directory, 'commit-tree', tree, '-p', parent],
                                          input=b'raw path\n').decode().strip()
            names.append(os.fsdecode(b'raw\xff.txt'))
            bound = dict(base=base, head=head, branch='harness/x')
            previous_cwd = os.getcwd()
            try:
                os.chdir(directory)
                packet = dev.review_packet(bound, extra_paths=[names[0]])
            finally:
                os.chdir(previous_cwd)
            for name in names:
                self.assertEqual(packet['files']['head:' + name], 'content\n')
