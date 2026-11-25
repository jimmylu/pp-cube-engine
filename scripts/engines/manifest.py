"""The live-compiled execution manifest of pp-cube-conformance, shared by the engine tiers."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CASE_KEYS = {'name', 'scenario', 'fixture', 'sql', 'params', 'columns', 'rows'}
# What a comparison against Cube Core needs besides the compiled SQL.
SOURCE_KEYS = {'query', 'timezone', 'security_filters'}
# How Cube Core answers a case, when not with the kernel's SQL: it refuses the query,
# or its SQL differs. Stated by the case; absent means equal.
RELATION_KEY = 'cube_core'
RELATIONS = ('refuses', 'differs')


def emitter(dialect):
    return ('cargo', 'run', '-q', '--locked', '-p', 'pp-cube-conformance',
            '--bin', 'execution-cases', '--', dialect)


def validated(manifest, dialect, case_keys):
    cases = manifest.get('cases')
    if manifest.get('schema') != 1 or manifest.get('dialect') != dialect or not cases:
        raise ValueError('unsupported execution manifest')
    if any(set(case) != case_keys or not case['fixture'] for case in cases):
        raise ValueError('malformed execution case')
    session = manifest.get('session', {})
    if not isinstance(session, dict) or any(
            not isinstance(value, str) or not re.fullmatch(r'[A-Za-z_.]+', name)
            or not re.fullmatch(r'[A-Za-z0-9_.]+', value) for name, value in session.items()):
        raise ValueError('unsupported execution manifest')
    return cases, session


def compiled_manifest(dialect, emit=subprocess.run):
    """Compiles the conformance cases now; the kernel itself never executes them.

    Returns the cases and the session properties their SQL needs on the engine."""
    output = emit(emitter(dialect), cwd=ROOT, text=True, stdout=subprocess.PIPE, timeout=900, check=True)
    return validated(json.loads(output.stdout), dialect, CASE_KEYS)


def compiled_sources(dialect, emit=subprocess.run):
    """The manifest with each case's Cube query, timezone and security filters, the
    model text and its data sources: what Cube Core is asked the same question with.
    Every case comes back with its relation to Cube Core: `equal` unless it states one."""
    output = emit(emitter(dialect) + ('--sources',), cwd=ROOT, text=True, stdout=subprocess.PIPE,
                  timeout=900, check=True)
    manifest = json.loads(output.stdout)
    listed = manifest.get('cases')
    if not isinstance(listed, list) or any(not isinstance(case, dict) for case in listed):
        raise ValueError('unsupported execution manifest')
    relations = [case.pop(RELATION_KEY, 'equal') for case in listed]
    if any(relation not in ('equal',) + RELATIONS for relation in relations):
        raise ValueError('unsupported execution manifest')
    cases, session = validated(manifest, dialect, CASE_KEYS | SOURCE_KEYS)
    model, data_sources = manifest.get('model'), manifest.get('data_sources')
    if (not isinstance(model, str) or not model.strip() or not isinstance(data_sources, list)
            or not data_sources
            or any(not isinstance(source, str) or not re.fullmatch(r'[a-z][a-z0-9_]*', source)
                   for source in data_sources)
            or any(not isinstance(case['query'], dict) or not isinstance(case['timezone'], str)
                   or not isinstance(case['security_filters'], list) for case in cases)):
        raise ValueError('unsupported execution manifest')
    for case, relation in zip(cases, relations):
        case[RELATION_KEY] = relation
    return cases, session, model, data_sources


def scenarios(cases):
    return ', '.join(sorted({case['scenario'] for case in cases}))
