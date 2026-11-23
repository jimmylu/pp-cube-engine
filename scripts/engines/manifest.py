"""The live-compiled execution manifest of pp-cube-conformance, shared by the engine tiers."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CASE_KEYS = {'name', 'scenario', 'fixture', 'sql', 'params', 'columns', 'rows'}


def emitter(dialect):
    return ('cargo', 'run', '-q', '--locked', '-p', 'pp-cube-conformance',
            '--bin', 'execution-cases', '--', dialect)


def compiled_manifest(dialect, emit=subprocess.run):
    """Compiles the conformance cases now; the kernel itself never executes them.

    Returns the cases and the session properties their SQL needs on the engine."""
    output = emit(emitter(dialect), cwd=ROOT, text=True, stdout=subprocess.PIPE, timeout=900, check=True)
    manifest = json.loads(output.stdout)
    cases = manifest.get('cases')
    if manifest.get('schema') != 1 or manifest.get('dialect') != dialect or not cases:
        raise ValueError('unsupported execution manifest')
    if any(set(case) != CASE_KEYS or not case['fixture'] for case in cases):
        raise ValueError('malformed execution case')
    session = manifest.get('session', {})
    if not isinstance(session, dict) or any(
            not isinstance(value, str) or not re.fullmatch(r'[a-z_.]+', name)
            or not re.fullmatch(r'[A-Za-z0-9_.]+', value) for name, value in session.items()):
        raise ValueError('unsupported execution manifest')
    return cases, session


def scenarios(cases):
    return ', '.join(sorted({case['scenario'] for case in cases}))
