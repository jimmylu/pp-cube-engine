"""A suite of questions: compile each, execute it, compare, and report."""
from dataclasses import dataclass, field
import datetime
import json
from pathlib import Path
import time
from typing import Any, Dict, List, Optional

import grpc

from ppcube import CompileRefused

from .engines import EngineError


@dataclass
class Case:
    name: str
    title: str
    query: dict
    row_filters: List[dict] = field(default_factory=list)
    expect: Optional[dict] = None
    refused: Optional[dict] = None


@dataclass
class Outcome:
    case: Case
    status: str = 'error'
    """pass, fail, refused (as expected), shown (no expectation: compare by hand) or error."""
    note: str = ''
    compiled: Any = None
    rows: Optional[List[List[Any]]] = None
    compile_ms: float = 0.0
    execute_ms: float = 0.0


def from_file(path: Path) -> List[Case]:
    document = json.loads(path.read_text(encoding='utf-8'))
    cases = []
    for item in document.get('cases', []):
        unknown = set(item) - {'name', 'title', 'query', 'rowFilters', 'expect', 'refused', 'about'}
        if unknown or 'name' not in item or 'query' not in item:
            raise ValueError('%s: a case needs name and query, and may have title, rowFilters, expect, refused'
                             % path.name)
        cases.append(Case(item['name'], item.get('title', item['name']), item['query'],
                          item.get('rowFilters', []), item.get('expect'), item.get('refused')))
    if not cases:
        raise ValueError('%s holds no cases' % path.name)
    return cases


def insurance(repository: Path, kit: Path, dialect: str) -> List[Case]:
    """The questions of examples/insurance-analytics, each user's under that user's
    rules, with what this dialect answers differently from Presto."""
    portal = json.loads((repository / 'examples' / 'insurance-analytics' / 'portal.json')
                        .read_text(encoding='utf-8'))
    differences = json.loads((kit / 'suites' / 'insurance-dialects.json').read_text(encoding='utf-8'))
    differences = differences.get(dialect, {})
    cases, seen = [], set()
    for user in portal['users']:
        for question in user['questions']:
            name = '%s/%s' % (user['name'], question['name'])
            seen.add(name)
            case = Case(name, question['title'], question['query'], user['rowFilters'],
                        question.get('expect'), question.get('refused'))
            if name in differences:
                case.expect = differences[name].get('expect')
                case.refused = differences[name].get('refused')
            cases.append(case)
    unknown = set(differences) - seen
    if unknown:
        raise ValueError('insurance-dialects.json names questions the example does not have: '
                         + ', '.join(sorted(unknown)))
    return cases


def difference(expect: dict, compiled, rows: List[List[Any]]) -> Optional[str]:
    """None when the rows are the expected ones; otherwise what differs."""
    aliases = [column.alias for column in compiled.columns]
    if aliases != expect['columns']:
        return 'columns %s, expected %s' % (aliases, expect['columns'])
    if len(rows) != len(expect['rows']):
        return '%d rows, expected %d' % (len(rows), len(expect['rows']))
    for number, (row, wanted) in enumerate(zip(rows, expect['rows']), 1):
        for alias, got, want in zip(aliases, row, wanted):
            if isinstance(want, dict):
                same = got == want['timestamp']
            elif isinstance(want, bool) or want is None or isinstance(want, str):
                same = got == want
            else:
                same = isinstance(got, float) and abs(got - want) < 1e-9
            if not same:
                return 'row %d, %s: %r, expected %r' % (number, alias, got, want)
    return None


def run(case: Case, kernel, executor, model_version: str, dialect: str, timezone: str) -> Outcome:
    outcome = Outcome(case)
    started = time.monotonic()
    try:
        outcome.compiled = kernel.compile(model_version, dialect, case.query, timezone=timezone,
                                          security_filters=case.row_filters, request_id='itest/' + case.name)
    except CompileRefused as refused:
        outcome.compile_ms = (time.monotonic() - started) * 1000
        kind = refused.kind.upper()
        outcome.note = 'no SQL (%s): %s' % (kind, refused)
        if case.refused and case.refused.get('kind') == kind:
            outcome.status = 'refused'
        elif case.refused or case.expect:
            outcome.status = 'fail'
        else:
            # Nothing was expected of it: a person decides whether this refusal is right.
            outcome.status = 'shown'
        return outcome
    except grpc.RpcError as failed:
        outcome.note = 'the call failed: %s %s' % (failed.code().name, failed.details() or '')
        return outcome
    outcome.compile_ms = (time.monotonic() - started) * 1000
    if case.refused:
        outcome.status, outcome.note = 'fail', 'expected no SQL (%s), got SQL' % case.refused.get('kind')
    started = time.monotonic()
    try:
        outcome.rows = executor.run(outcome.compiled)
    except EngineError as error:
        outcome.execute_ms = (time.monotonic() - started) * 1000
        outcome.status, outcome.note = 'error', 'the engine refused the SQL: %s' % error
        return outcome
    outcome.execute_ms = (time.monotonic() - started) * 1000
    if case.refused:
        return outcome
    if case.expect is None:
        outcome.status = 'shown'
        return outcome
    differs = difference(case.expect, outcome.compiled, outcome.rows)
    outcome.status, outcome.note = ('fail', differs) if differs else ('pass', '')
    return outcome


def shown(value: Any) -> str:
    if value is None:
        return '-'
    if isinstance(value, float):
        return format(int(value), ',d') if value == int(value) else format(value, ',.4f')
    return str(value)


def table(compiled, rows: List[List[Any]], limit: int) -> List[str]:
    lines = [[column.member for column in compiled.columns]] + [[shown(v) for v in row] for row in rows[:limit]]
    widths = [max(len(line[index]) for line in lines) for index in range(len(lines[0]))]
    text = [''.join(cell.ljust(width + 2) for cell, width in zip(line, widths)).rstrip() for line in lines]
    if len(rows) > limit:
        text.append('… %d more rows' % (len(rows) - limit))
    return text


def counts(outcomes: List[Outcome]) -> Dict[str, int]:
    return {status: sum(o.status == status for o in outcomes)
            for status in ('pass', 'refused', 'shown', 'fail', 'error')}


def summary(outcomes: List[Outcome]) -> str:
    c = counts(outcomes)
    return ('%d cases: %d passed, %d refused as expected, %d shown for comparison by hand, %d failed, %d errors'
            % (len(outcomes), c['pass'], c['refused'], c['shown'], c['fail'], c['error']))


def report(outcomes: List[Outcome], facts: Dict[str, str], directory: Path) -> Path:
    """`report.md` for people and `report.json` for tools. Neither holds a token or a password."""
    directory.mkdir(parents=True, exist_ok=True)
    fence = '```'
    lines = ['# Integration test report', '',
             '| | |', '| --- | --- |'] + ['| %s | %s |' % item for item in facts.items()]
    lines += ['', summary(outcomes), '', '| Case | Status | Compile ms | Execute ms | Rows | Note |',
              '| --- | --- | --- | --- | --- | --- |']
    for o in outcomes:
        lines.append('| %s | %s | %.0f | %.0f | %s | %s |' % (
            o.case.name, o.status.upper(), o.compile_ms, o.execute_ms,
            '' if o.rows is None else len(o.rows), o.note.replace('|', '\\|')))
    for o in outcomes:
        lines += ['', '## %s' % o.case.name, '', o.case.title, '', '**%s** %s' % (o.status.upper(), o.note), '',
                  'Query:', '', fence + 'json', json.dumps(o.case.query, ensure_ascii=False), fence]
        if o.case.row_filters:
            lines += ['', 'Security filters:', '', fence + 'json',
                      json.dumps(o.case.row_filters, ensure_ascii=False), fence]
        if o.compiled is not None:
            lines += ['', 'SQL:', '', fence + 'sql', o.compiled.sql, fence, '',
                      'Parameters: `%s`' % json.dumps(list(o.compiled.parameters), ensure_ascii=False), '',
                      'Session properties: `%s`' % json.dumps(o.compiled.session_properties), '',
                      'Fingerprint: `%s`' % o.compiled.fingerprint]
        if o.rows is not None:
            lines += ['', 'Rows:', '', fence + 'text'] + table(o.compiled, o.rows, 50) + [fence]
    (directory / 'report.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    document = {'facts': facts, 'counts': counts(outcomes), 'cases': [{
        'name': o.case.name, 'status': o.status, 'note': o.note, 'query': o.case.query,
        'security_filters': o.case.row_filters,
        'sql': o.compiled.sql if o.compiled else None,
        'parameters': list(o.compiled.parameters) if o.compiled else None,
        'session_properties': o.compiled.session_properties if o.compiled else None,
        'fingerprint': o.compiled.fingerprint if o.compiled else None,
        'compile_ms': round(o.compile_ms, 1), 'execute_ms': round(o.execute_ms, 1), 'rows': o.rows,
    } for o in outcomes]}
    (directory / 'report.json').write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n',
                                           encoding='utf-8')
    return directory / 'report.md'


def stamp() -> str:
    return datetime.datetime.now().astimezone().strftime('%Y-%m-%d %H:%M:%S %z')
