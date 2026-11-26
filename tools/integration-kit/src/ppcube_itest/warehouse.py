"""The benchmark data as statements for each engine.

The rows are those of examples/insurance-analytics/warehouse/insurance.json;
the expected answers of the built-in suite were worked out from them by hand.
Only the nine tables named there are ever created, replaced or dropped, in the
schema and with the prefix the configuration names.
"""
import json
from pathlib import Path
from typing import Any, Dict, List

PRESTO_TYPES = {'integer': 'INTEGER', 'string': 'VARCHAR', 'double': 'DOUBLE', 'timestamp': 'TIMESTAMP'}
DORIS_TYPES = {'integer': 'INT', 'string': 'VARCHAR(64)', 'double': 'DOUBLE', 'timestamp': 'DATETIMEV2(3)'}
DRUID_TYPES = {'integer': 'long', 'string': 'string', 'double': 'double', 'timestamp': 'string'}


def tables(repository: Path) -> List[Dict[str, Any]]:
    path = repository / 'examples' / 'insurance-analytics' / 'warehouse' / 'insurance.json'
    return json.loads(path.read_text(encoding='utf-8'))['tables']


def _text(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def _literal(value, kind: str, timestamp) -> str:
    if value is None:
        return 'NULL'
    if kind == 'timestamp':
        return timestamp(value.replace('T', ' '))
    if kind == 'string':
        return _text(value)
    return repr(value)


def _insert(qualified: str, table, timestamp) -> str:
    rows = ['(' + ','.join(_literal(value, column['type'], timestamp)
                           for value, column in zip(row, table['columns'])) + ')'
            for row in table['rows']]
    return 'INSERT INTO %s VALUES %s' % (qualified, ','.join(rows))


def presto(table, schema: str, name: str) -> List[str]:
    qualified = '%s.%s' % (schema, name)
    columns = ', '.join('%s %s' % (c['name'], PRESTO_TYPES[c['type']]) for c in table['columns'])
    return ['CREATE TABLE %s (%s)' % (qualified, columns),
            _insert(qualified, table, lambda value: "TIMESTAMP '%s'" % value)]


def doris(table, schema: str, name: str, replication: int) -> List[str]:
    qualified = '%s.%s' % (schema, name)
    key = table['columns'][0]['name']
    columns = ', '.join('%s %s' % (c['name'], DORIS_TYPES[c['type']]) for c in table['columns'])
    return ["CREATE TABLE %s (%s) DUPLICATE KEY(%s) DISTRIBUTED BY HASH(%s) BUCKETS 1 "
            "PROPERTIES ('replication_num'='%d')" % (qualified, columns, key, key, replication),
            _insert(qualified, table, lambda value: "'%s'" % value)]


def druid(table, name: str) -> str:
    """One SQL ingestion that replaces the datasource with the rows, written inline.

    A datasource has one timestamp, `__time`: the table's `time` column becomes
    it, and a table without one gets a constant. Other timestamps stay text."""
    time = table.get('time')
    selected = []
    for column in table['columns']:
        if column['name'] == time:
            selected.append('TIME_PARSE("%s") AS "__time"' % time)
        else:
            selected.append('"%s"' % column['name'])
    if not time:
        selected.insert(0, "TIMESTAMP '2000-01-01 00:00:00' AS \"__time\"")
    data = '\n'.join(json.dumps(dict(zip([c['name'] for c in table['columns']], row)),
                                separators=(',', ':')) for row in table['rows'])
    source = json.dumps({'type': 'inline', 'data': data}, separators=(',', ':'))
    signature = json.dumps([{'name': c['name'], 'type': DRUID_TYPES[c['type']]} for c in table['columns']],
                           separators=(',', ':'))
    return ('REPLACE INTO "%s" OVERWRITE ALL SELECT %s FROM TABLE(EXTERN(%s, \'{"type":"json"}\', %s)) '
            'PARTITIONED BY ALL' % (name, ', '.join(selected), _text(source), _text(signature)))
