"""Real local Docker execution of a live-compiled execution manifest read from stdin."""
import datetime
import importlib.util
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('doris', root / 'scripts/engines/doris-container.py')
doris = importlib.util.module_from_spec(spec)
spec.loader.exec_module(doris)
manifest = json.load(sys.stdin)
if manifest.get('schema') != 1 or manifest.get('dialect') != 'doris' or not manifest.get('cases'):
    sys.exit('unsupported execution manifest')


def parameter(item):
    kind = item['type']
    if kind == 'null':
        return None
    if kind in ('text', 'number', 'bool'):
        return item['value']
    raise ValueError('unsupported parameter type')


def cell(value):
    if isinstance(value, dict):
        if list(value) != ['timestamp']:
            raise ValueError('unsupported expected cell')
        return datetime.datetime.fromisoformat(value['timestamp'])
    return value


def suite(connection):
    loaded = None
    for case in manifest['cases']:
        if case['fixture'] != loaded:
            cursor = connection.cursor()
            for statement in case['fixture']:
                cursor.execute(statement)
            cursor.close()
            loaded = case['fixture']
        # Parameters travel separately through the native prepared protocol.
        cursor = connection.cursor(prepared=True)
        try:
            doris.verify(cursor, case['sql'], tuple(parameter(item) for item in case['params']),
                         case['columns'], [tuple(cell(value) for value in row) for row in case['rows']])
        finally:
            cursor.close()


doris.suite = suite
doris.run()
print('Doris live-compiled manifest execution: PASSED — %d cases' % len(manifest['cases']))
