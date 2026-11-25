"""The release helper on a throwaway repository; nothing is built or published."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).parents[2]
CHANGELOG = '''# Changelog

## Unreleased

- Not out yet.

## 0.2.0

**Compiles**
- A second thing.

## 0.1.0

- A first thing.
'''


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.directory)
        (self.directory / 'scripts').mkdir()
        shutil.copy(ROOT / 'scripts/release.sh', self.directory / 'scripts/release.sh')
        (self.directory / 'Cargo.toml').write_text(
            '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.2.0"\nedition = "2021"\n\n'
            '[workspace.dependencies]\nother = { version = "9.9.9" }\n')
        (self.directory / 'CHANGELOG.md').write_text(CHANGELOG)

    def run_release(self, *arguments, env=None):
        return subprocess.run(['bash', 'scripts/release.sh', *arguments], cwd=self.directory, text=True,
                              capture_output=True, env={'PATH': '/usr/bin:/bin:/usr/local/bin', **(env or {})})

    def test_the_version_is_the_workspace_version(self):
        result = self.run_release('version')
        self.assertEqual((result.returncode, result.stdout), (0, '0.2.0\n'))
        (self.directory / 'Cargo.toml').write_text('[workspace.package]\nversion = "next"\n')
        result = self.run_release('version')
        self.assertEqual(result.returncode, 1)
        self.assertIn('release: FAILED', result.stderr)

    def test_notes_are_one_version_section(self):
        result = self.run_release('notes', '0.2.0')
        self.assertEqual(result.stdout, '**Compiles**\n- A second thing.\n')
        self.assertEqual(self.run_release('notes', '0.1.0').stdout, '- A first thing.\n')
        # A prefix of a version is not that version, and a missing one fails.
        for missing in ('0.2', '0.3.0'):
            self.assertEqual(self.run_release('notes', missing).returncode, 1, missing)

    def test_a_tag_must_be_the_version_with_a_changelog_section(self):
        skip = {'RELEASE_SKIP_MAIN_CHECK': '1'}
        result = self.run_release('check', 'v0.2.0', env=skip)
        self.assertEqual((result.returncode, result.stdout), (0, 'release: v0.2.0\n'))
        for tag in ('0.2.0', 'v0.2.1', 'v0.2.0-rc1', ''):
            result = self.run_release('check', tag, env=skip)
            self.assertEqual(result.returncode, 1, tag)
            self.assertIn('the tag must be v0.2.0', result.stderr)
        (self.directory / 'CHANGELOG.md').write_text('# Changelog\n\n## Unreleased\n\n- Only this.\n')
        result = self.run_release('check', 'v0.2.0', env=skip)
        self.assertIn("no section '## 0.2.0'", result.stderr)

    def test_a_release_is_cut_from_main(self):
        def git(*arguments):
            subprocess.run(['git', '-c', 'user.name=t', '-c', 'user.email=t@example.test', *arguments],
                           cwd=self.directory, check=True, capture_output=True)
        git('init', '-q', '-b', 'main')
        git('add', '.')
        git('commit', '-q', '-m', 'one')
        git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.assertEqual(self.run_release('check', 'v0.2.0').returncode, 0)
        git('checkout', '-q', '-b', 'side')
        git('commit', '-q', '--allow-empty', '-m', 'two')
        result = self.run_release('check', 'v0.2.0')
        self.assertEqual(result.returncode, 1)
        self.assertIn('the tagged commit is not on main', result.stderr)

    def test_the_image_is_the_repository_in_its_own_registry(self):
        self.assertEqual(self.run_release('image', 'Example/Repo.Name').stdout, 'ghcr.io/example/repo.name\n')
        for bad in ('example', 'example/repo/extra', 'example/re po', ''):
            self.assertEqual(self.run_release('image', bad).returncode, 1, bad)

    def test_a_build_refuses_what_it_was_not_given_exactly(self):
        revision = 'a' * 40
        for arguments in [
                ('docker.io/example/repo', '0.2.0', 'amd64', revision),
                ('ghcr.io/Example/repo', '0.2.0', 'amd64', revision),
                ('ghcr.io/example/repo', 'v0.2.0', 'amd64', revision),
                ('ghcr.io/example/repo', '0.2.0', 'riscv', revision),
                ('ghcr.io/example/repo', '0.2.0', 'amd64', 'abc'),
                ('ghcr.io/example/repo', '0.2.0', 'amd64', revision, 'latest'),
                # Everything valid, but no upstream sources to build from.
                ('ghcr.io/example/repo', '0.2.0', 'amd64', revision)]:
            result = self.run_release('build', *arguments)
            self.assertEqual(result.returncode, 1, arguments)
            self.assertIn('release: FAILED', result.stderr)
            self.assertEqual(result.stdout, '')

    def test_the_workflow_publishes_only_from_a_tag(self):
        workflow = (ROOT / '.github/workflows/release.yml').read_text()
        publish = workflow.split('\n  publish:\n', 1)[1]
        self.assertIn("if: needs.prepare.outputs.publish == 'true'", publish)
        # Write permissions are a job's own, never the workflow's.
        self.assertIn('\npermissions:\n  contents: read\n', workflow)
        before = workflow.split('\n  publish:\n', 1)[0]
        for permission in ('contents: write', 'id-token: write', 'attestations: write'):
            self.assertNotIn(permission, before)
            self.assertIn(permission, publish)
        # No mutable tag, and no image name written into the file.
        self.assertNotIn(':latest', workflow)
        self.assertNotIn('ghcr.io/', workflow.replace('docker login ghcr.io', ''))


if __name__ == '__main__':
    unittest.main()
