"""The benchmark model as a version of a model store, for one engine.

The model is the one of examples/insurance-analytics, one cube or view a file.
What changes for an engine is where its tables are: the schema, the prefix and
the data source of the configuration, and on Druid the timestamp column."""
from pathlib import Path
import re
from typing import Dict

from .config import Engine

SOURCE = Path('examples') / 'insurance-analytics' / 'model-store' / 'property' / 'models' / '2026-11'
TABLE = re.compile(r'^(\s*sql_table: )dw\.(ins_[a-z_]+)$', re.MULTILINE)
SOURCE_NAME = re.compile(r'^(\s*data_source: )warehouse$', re.MULTILINE)


def placed(model_version: str):
    """`itest/presto/v1` is version `v1` of business line `itest/presto`."""
    line, _, version = model_version.rpartition('/')
    if not line or not version:
        raise ValueError('model_version must be <business line>/<version>, such as itest/presto/v1')
    return line, version


def render(repository: Path, engine: Engine, time_columns: Dict[str, str]) -> Dict[str, str]:
    """The files of the version, by their path inside it."""
    files = {}
    for folder in ('cubes', 'views'):
        for path in sorted((repository / SOURCE / folder).glob('*.yml')):
            text = path.read_text(encoding='utf-8')
            if folder == 'cubes':
                text, tables = TABLE.subn(
                    lambda match: '%s%s.%s' % (match.group(1), engine.schema, engine.table(match.group(2))), text)
                text, sources = SOURCE_NAME.subn(lambda match: match.group(1) + engine.data_source, text)
                if tables != 1 or sources != 1:
                    # A cube this does not understand must not be published half rewritten.
                    raise ValueError('%s: expected one sql_table in dw and one data_source' % path.name)
                if engine.dialect == 'druid':
                    for column in time_columns.values():
                        text = text.replace('{CUBE}.%s"' % column, '{CUBE}.__time"')
            files['%s/%s' % (folder, path.name)] = text
    files['manifest.json'] = '{"schema": 1}\n'
    return files


def write(files: Dict[str, str], out: Path, model_version: str) -> Path:
    line, version = placed(model_version)
    root = out.joinpath(*line.split('/')) / 'models' / version
    if root.exists():
        raise FileExistsError('%s exists; a published version is never rewritten' % root)
    # The manifest goes last: a version is offered only once it is there.
    for name in sorted(files, key=lambda name: name == 'manifest.json'):
        target = root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(files[name], encoding='utf-8')
    return root
