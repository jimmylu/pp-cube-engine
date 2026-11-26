"""`ppcube-itest`: the commands of docs/integration-testing.zh-CN.md.

Run it from a checkout of the repository: the benchmark model, data and
questions are the example's, read from there.
"""
import argparse
import json
import os
from pathlib import Path
import sys
import time

import grpc

from ppcube import Client, CompileRefused

from . import __version__, config, engines, modelstore, suite, warehouse

KIT = Path('tools') / 'integration-kit'


def _client(service: config.Service) -> Client:
    return Client(service.target, token=service.token(), trust=service.ca,
                  plaintext=service.plaintext, deadline=service.deadline)


def _json(path: str):
    text = sys.stdin.read() if path == '-' else Path(path).read_text(encoding='utf-8')
    return json.loads(text)


def _print_compiled(case_query, filters, compiled) -> None:
    print('request')
    print('  query:            ' + json.dumps(case_query, ensure_ascii=False))
    print('  security filters: ' + json.dumps(filters, ensure_ascii=False))
    print('sql')
    for line in compiled.sql.splitlines():
        print('  ' + line)
    print('parameters:         ' + json.dumps(list(compiled.parameters), ensure_ascii=False))
    print('session properties: ' + json.dumps(compiled.session_properties))
    print('columns:            ' + ', '.join('%s=%s(%s)' % (c.alias, c.member, c.type) for c in compiled.columns))
    print('fingerprint:        ' + compiled.fingerprint)
    print('model:              %s %s, kernel %s' % (compiled.model_version, compiled.model_digest,
                                                    compiled.kernel_version))


def env(arguments, settings) -> int:
    """Is everything this test needs reachable, and what is it?"""
    failed = 0
    chosen = [settings.engine(arguments.engine)] if arguments.engine else list(settings.engines.values())
    print('service %s' % settings.service.target)
    with _client(settings.service) as kernel:
        for engine in chosen:
            print('engine %s (%s)' % (engine.name, engine.dialect))
            try:
                meta = kernel.meta(engine.model_version)
                print('  model    %s %s, kernel %s, %d cubes and views' % (
                    meta.identity.model_version, meta.identity.model_digest[:19],
                    meta.identity.kernel_version, len(meta.cubes)))
                sources = sorted({cube.data_source for cube in meta.cubes})
                if sources != [engine.data_source]:
                    print('  model    WARNING: its data sources are %s, the configuration says %s'
                          % (sources, engine.data_source))
            except grpc.RpcError as error:
                failed += 1
                print('  model    FAILED: %s %s' % (error.code().name, error.details() or ''))
            try:
                executor = engines.connect(engine)
                print('  engine   ' + executor.version())
                present = executor.existing()
                names = [engine.table(table['name']) for table in warehouse.tables(arguments.repo)]
                found = [name for name in names if name in present]
                print('  tables   %d of %d benchmark tables in %s' % (len(found), len(names), engine.schema))
            except engines.EngineError as error:
                failed += 1
                print('  engine   FAILED: %s' % error)
    print('env: ' + ('PASSED' if not failed else 'FAILED, %d problems' % failed))
    return 1 if failed else 0


def meta(arguments, settings) -> int:
    """The cubes, views and members of the engine's model version."""
    engine = settings.engine(arguments.engine)
    with _client(settings.service) as kernel:
        described = kernel.meta(engine.model_version)
    print('%s %s, kernel %s' % (described.identity.model_version, described.identity.model_digest,
                                described.identity.kernel_version))
    for cube in described.cubes:
        print('%s  (data source %s)' % (cube.name, cube.data_source))
        for measure in cube.measures:
            print('  measure    %s.%s  %s' % (cube.name, measure.name, measure.type))
        for dimension in cube.dimensions:
            print('  dimension  %s.%s  %s' % (cube.name, dimension.name, dimension.type))
        for segment in cube.segments:
            print('  segment    %s.%s' % (cube.name, segment))
    return 0


def _compile(arguments, settings):
    engine = settings.engine(arguments.engine)
    query = _json(arguments.query)
    filters = _json(arguments.filters) if arguments.filters else []
    if not isinstance(filters, list):
        raise ValueError('--filters is a JSON list of Cube filters')
    with _client(settings.service) as kernel:
        started = time.monotonic()
        compiled = kernel.compile(engine.model_version, engine.dialect, query, timezone=arguments.timezone,
                                  security_filters=filters, request_id='itest/cli')
        elapsed = (time.monotonic() - started) * 1000
    _print_compiled(query, filters, compiled)
    print('compiled in:        %.0f ms' % elapsed)
    return engine, compiled


def compile_(arguments, settings) -> int:
    """A Cube query to SQL: the request, the SQL, its parameters and what else a caller needs."""
    _compile(arguments, settings)
    return 0


def run(arguments, settings) -> int:
    """Compile, then execute on the engine as a caller would, and show the rows."""
    engine, compiled = _compile(arguments, settings)
    started = time.monotonic()
    rows = engines.connect(engine).run(compiled)
    print('executed in:        %.0f ms, %d rows' % ((time.monotonic() - started) * 1000, len(rows)))
    for line in suite.table(compiled, rows, arguments.max_rows):
        print('  ' + line)
    return 0


def model(arguments, settings) -> int:
    """Write the benchmark model as a version of a model store, for the engine."""
    engine = settings.engine(arguments.engine)
    times = {table['name']: table['time'] for table in warehouse.tables(arguments.repo) if 'time' in table}
    files = modelstore.render(arguments.repo, engine, times)
    root = modelstore.write(files, Path(arguments.out), engine.model_version)
    print('wrote %d files to %s' % (len(files), root))
    print('copy the directory tree under %s into the model store; the version is %s' % (
        arguments.out, engine.model_version))
    return 0


def load(arguments, settings) -> int:
    """Create the benchmark tables on the engine and load their rows."""
    engine = settings.engine(arguments.engine)
    tables = warehouse.tables(arguments.repo)
    if arguments.dry_run:
        # For whoever has the right to run them, when the tester has not.
        for statement in engines.statements(engine, tables):
            print(statement + ';')
        return 0
    executor = engines.connect(engine)
    names = {engine.table(table['name']) for table in tables}
    present = names & executor.existing()
    if present and not arguments.replace:
        print('load: %s already holds %s' % (engine.schema, ', '.join(sorted(present))))
        print('load: nothing was changed; pass --replace to drop and load these tables again')
        return 1
    for table in tables:
        name = engine.table(table['name'])
        if name in present and engine.dialect != 'druid':
            executor.drop(name)
        executor.load(table)
        print('loaded %s.%s, %d rows' % (engine.schema, name, len(table['rows'])))
    print('load: PASSED, %d tables' % len(tables))
    return 0


def clean(arguments, settings) -> int:
    """Remove the benchmark tables, and nothing else, from the engine."""
    engine = settings.engine(arguments.engine)
    if not arguments.yes:
        print('clean: pass --yes to drop the benchmark tables of %s in %s' % (engine.name, engine.schema))
        return 1
    executor = engines.connect(engine)
    present = executor.existing()
    for table in warehouse.tables(arguments.repo):
        name = engine.table(table['name'])
        if name in present:
            executor.drop(name)
            print('dropped %s.%s' % (engine.schema, name))
    print('clean: PASSED')
    return 0


def suite_(arguments, settings) -> int:
    """Run a suite of questions and write a report."""
    engine = settings.engine(arguments.engine)
    if arguments.builtin:
        cases = suite.insurance(arguments.repo, arguments.repo / KIT, engine.dialect)
    else:
        cases = suite.from_file(Path(arguments.file))
    executor = engines.connect(engine)
    outcomes = []
    with _client(settings.service) as kernel:
        described = kernel.meta(engine.model_version)
        facts = {
            'When': suite.stamp(),
            'Service': settings.service.target,
            'Kernel': described.identity.kernel_version,
            'Model version': '%s %s' % (described.identity.model_version, described.identity.model_digest),
            'Engine': '%s (%s): %s' % (engine.name, engine.dialect, executor.version()),
            'Suite': 'built-in insurance' if arguments.builtin else arguments.file,
            'Timezone': arguments.timezone,
        }
        for case in cases:
            outcome = suite.run(case, kernel, executor, engine.model_version, engine.dialect, arguments.timezone)
            outcomes.append(outcome)
            print('%-8s %s%s' % (outcome.status.upper(), case.name, '  ' + outcome.note if outcome.note else ''))
    print(suite.summary(outcomes))
    if arguments.report:
        print('report: %s' % suite.report(outcomes, facts, Path(arguments.report)))
    bad = sum(outcome.status in ('fail', 'error') for outcome in outcomes)
    print('suite: ' + ('PASSED' if not bad else 'FAILED'))
    return 1 if bad else 0


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(prog='ppcube-itest', description=__doc__.split('\n')[0])
    top.add_argument('--version', action='version', version='ppcube-itest ' + __version__)
    top.add_argument('--config', default=os.environ.get('PPCUBE_ITEST_CONFIG', 'itest.ini'),
                     help='the environment under test (default: itest.ini, or $PPCUBE_ITEST_CONFIG)')
    top.add_argument('--repo', type=Path, default=Path.cwd(),
                     help='a checkout of the repository (default: the current directory)')
    commands = top.add_subparsers(dest='command', required=True)

    def command(name, function, engine=True):
        sub = commands.add_parser(name, help=function.__doc__)
        sub.set_defaults(function=function)
        if engine:
            sub.add_argument('--engine', help='an [engine:<name>] of the configuration; optional when there is one')
        return sub

    command('env', env)
    command('meta', meta)
    for name, function in (('compile', compile_), ('run', run)):
        sub = command(name, function)
        sub.add_argument('--query', required=True, help='a Cube query as a JSON file, or - for standard input')
        sub.add_argument('--filters', help='security filters: a JSON file holding a list of Cube filters')
        sub.add_argument('--timezone', default='UTC')
    commands.choices['run'].add_argument('--max-rows', type=int, default=50, help='rows to show (default 50)')
    command('model', model).add_argument('--out', required=True, help='a directory to write the store layout into')
    sub = command('load', load)
    sub.add_argument('--replace', action='store_true', help='drop and load benchmark tables that already exist')
    sub.add_argument('--dry-run', action='store_true',
                     help='print the statements instead of running them, for someone else to run')
    command('clean', clean).add_argument('--yes', action='store_true', help='really drop the benchmark tables')
    sub = command('suite', suite_)
    which = sub.add_mutually_exclusive_group(required=True)
    which.add_argument('--builtin', choices=['insurance'], help='the benchmark suite, with expected answers')
    which.add_argument('--file', help='a suite of your own; see suites/template.json')
    sub.add_argument('--report', help='a directory to write report.md and report.json into')
    sub.add_argument('--timezone', default='UTC')
    return top


def main(argv=None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if not (arguments.repo / 'examples' / 'insurance-analytics' / 'portal.json').is_file():
            raise config.ConfigError('Run from a checkout of the repository, or pass --repo: %s is not one'
                                     % arguments.repo)
        return arguments.function(arguments, config.load(arguments.config))
    except CompileRefused as refused:
        print('no SQL (%s): %s' % (refused.kind.upper(), refused))
        return 3
    except grpc.RpcError as failed:
        print('the call to the service failed: %s %s' % (failed.code().name, failed.details() or ''), file=sys.stderr)
        return 2
    except (config.ConfigError, engines.EngineError, ValueError, OSError) as error:
        print('%s: %s' % (arguments.command, error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
