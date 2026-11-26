"""One executor an engine: what a caller's back end does with compiled SQL.

Each runs the statement as the kernel wrote it, hands the parameters to the
driver apart from the text, applies the session properties, and returns rows
in the kernel's column order with values in one form whatever the driver gives.
"""
import base64
import datetime
import decimal
import json
import ssl
import time
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple
import urllib.error
import urllib.request

from . import warehouse
from .config import Engine


class EngineError(Exception):
    """The engine refused or could not be reached. The message is the engine's own,
    cut short: this kit is run by the people who operate the engine."""


def _short(text: Any, limit: int = 400) -> str:
    text = ' '.join(str(text).split())
    return text if len(text) <= limit else text[:limit] + ' …'


def normalized(value: Any, kind: str) -> Any:
    """A cell in the one form results are compared in: a time as local
    `YYYY-MM-DDTHH:MM:SS`, a number as a float, anything else as it is."""
    if value is None:
        return None
    if kind == 'time':
        if isinstance(value, (datetime.datetime, datetime.date)):
            value = value.isoformat(sep='T') if isinstance(value, datetime.datetime) else value.isoformat() + 'T00:00:00'
        text = str(value).replace(' ', 'T')
        return text.rstrip('Z').split('.')[0]
    if kind == 'number':
        if isinstance(value, (int, float, decimal.Decimal)) and not isinstance(value, bool):
            return float(value)
        try:
            return float(value)
        except (TypeError, ValueError):
            return value
    if kind == 'boolean' and isinstance(value, int):
        return bool(value)
    return value


def arranged(compiled, names: Sequence[str], records: Iterable[Sequence[Any]]) -> List[List[Any]]:
    """Rows in the kernel's column order, read by the kernel's column names."""
    missing = [column.alias for column in compiled.columns if column.alias not in names]
    if missing:
        raise EngineError('The result has no column ' + ', '.join(missing))
    index = [(names.index(column.alias), column.type) for column in compiled.columns]
    return [[normalized(record[position], kind) for position, kind in index] for record in records]


class Presto:
    def __init__(self, engine: Engine):
        self.engine = engine

    def _connect(self, session: Optional[Dict[str, str]] = None):
        import prestodb
        engine, password = self.engine, self.engine.password()
        scheme = engine.option('http_scheme', 'https' if password else 'http')
        return prestodb.dbapi.connect(
            host=engine.option('host'), port=int(engine.option('port', '8080')),
            user=engine.option('user', 'ppcube-itest'), catalog=engine.option('catalog'),
            schema=engine.schema, http_scheme=scheme, session_properties=dict(session or {}),
            auth=prestodb.auth.BasicAuthentication(engine.option('user'), password) if password else None,
            request_timeout=float(engine.option('timeout', '120')))

    def _all(self, sql: str, parameters: Sequence[Any] = (), session=None) -> Tuple[List[str], List[Sequence[Any]]]:
        try:
            cursor = self._connect(session).cursor()
            cursor.execute(sql, list(parameters) or None)
            records = cursor.fetchall()
            return [column[0] for column in cursor.description or ()], records
        except Exception as error:  # the driver raises its own and the transport's
            raise EngineError(_short(getattr(error, 'message', None) or error)) from None

    def version(self) -> str:
        return str(self._all('SELECT node_version FROM system.runtime.nodes LIMIT 1')[1][0][0])

    def run(self, compiled) -> List[List[Any]]:
        names, records = self._all(compiled.sql, compiled.parameters, compiled.session_properties)
        return arranged(compiled, names, records)

    def existing(self) -> Set[str]:
        _, records = self._all("SELECT table_name FROM information_schema.tables WHERE table_schema = '%s'"
                               % self.engine.schema)
        return {record[0] for record in records}

    def load(self, table) -> None:
        self._all('CREATE SCHEMA IF NOT EXISTS ' + self.engine.schema)
        for statement in warehouse.presto(table, self.engine.schema, self.engine.table(table['name'])):
            self._all(statement)

    def drop(self, name: str) -> None:
        self._all('DROP TABLE IF EXISTS %s.%s' % (self.engine.schema, name))


class Doris:
    def __init__(self, engine: Engine):
        self.engine = engine

    def _all(self, sql: str, parameters: Sequence[Any] = (), session=None) -> Tuple[List[str], List[Sequence[Any]]]:
        import pymysql
        engine = self.engine
        try:
            connection = pymysql.connect(
                host=engine.option('host'), port=int(engine.option('port', '9030')),
                user=engine.option('user', 'root'), password=engine.password() or '',
                autocommit=True, charset='utf8mb4',
                connect_timeout=30, read_timeout=int(float(engine.option('timeout', '120'))))
            try:
                with connection.cursor() as cursor:
                    for name, value in (session or {}).items():
                        cursor.execute('SET %s = %%s' % name, (value,))
                    cursor.execute(sql, tuple(parameters) or None)
                    records = cursor.fetchall()
                    return [column[0] for column in cursor.description or ()], list(records)
            finally:
                connection.close()
        except pymysql.MySQLError as error:
            raise EngineError(_short(error)) from None

    def version(self) -> str:
        _, records = self._all("SHOW VARIABLES LIKE 'version_comment'")
        return records[0][1] if records else 'unknown'

    def run(self, compiled) -> List[List[Any]]:
        # This driver's placeholder is %s; the client converts, leaving quoted text alone.
        names, records = self._all(compiled.sql_as('format'), compiled.parameters, compiled.session_properties)
        return arranged(compiled, names, records)

    def existing(self) -> Set[str]:
        _, records = self._all('SELECT TABLE_NAME FROM information_schema.tables WHERE TABLE_SCHEMA = %s',
                               (self.engine.schema,))
        return {record[0] for record in records}

    def load(self, table) -> None:
        self._all('CREATE DATABASE IF NOT EXISTS ' + self.engine.schema)
        replication = int(self.engine.option('replication_num', '1'))
        for statement in warehouse.doris(table, self.engine.schema, self.engine.table(table['name']), replication):
            self._all(statement)

    def drop(self, name: str) -> None:
        self._all('DROP TABLE IF EXISTS %s.%s' % (self.engine.schema, name))


class Druid:
    TYPES = ((bool, 'BOOLEAN'), (str, 'VARCHAR'), (float, 'DOUBLE'), (int, 'BIGINT'))

    def __init__(self, engine: Engine, pause=time.sleep):
        self.engine = engine
        self.base = (engine.option('url') or '').rstrip('/')
        self.pause = pause
        self.timeout = float(engine.option('timeout', '120'))

    def _call(self, path: str, body: Optional[dict] = None, method: Optional[str] = None) -> Any:
        headers = {'Content-Type': 'application/json'}
        password = self.engine.password()
        if password:
            pair = '%s:%s' % (self.engine.option('user', ''), password)
            headers['Authorization'] = 'Basic ' + base64.b64encode(pair.encode('utf-8')).decode('ascii')
        context = ssl.create_default_context(cafile=self.engine.option('ca')) if self.base.startswith('https') else None
        request = urllib.request.Request(
            self.base + path, data=None if body is None else json.dumps(body).encode('utf-8'),
            headers=headers, method=method)
        try:
            with urllib.request.urlopen(request, timeout=self.timeout, context=context) as response:
                text = response.read().decode('utf-8')
                return json.loads(text) if text.strip() else None
        except urllib.error.HTTPError as error:
            text = error.read().decode('utf-8', 'replace')
            try:
                document = json.loads(text)
                text = document.get('errorMessage') or document.get('error') or text
            except ValueError:
                pass
            raise EngineError('HTTP %d: %s' % (error.code, _short(text))) from None
        except (OSError, ValueError) as error:
            raise EngineError(_short(error)) from None

    def _parameter(self, value: Any) -> dict:
        if value is None:
            return {'type': 'VARCHAR', 'value': None}
        for kind, name in self.TYPES:
            if isinstance(value, kind):
                return {'type': name, 'value': value}
        raise EngineError('A parameter of an unknown kind cannot be sent')

    def sql(self, query: str, parameters: Sequence[Any] = (), context: Optional[Dict[str, str]] = None):
        body = {'query': query, 'resultFormat': 'array', 'header': True, 'context': dict(context or {})}
        if parameters:
            body['parameters'] = [self._parameter(value) for value in parameters]
        result = self._call('/druid/v2/sql', body)
        if not isinstance(result, list) or not result:
            raise EngineError('Druid returned no header')
        return result[0], result[1:]

    def version(self) -> str:
        version = (self._call('/status') or {}).get('version', 'unknown')
        self.sql('SELECT 1')
        return '%s, NULL handling %s' % (version, self.null_handling())

    def null_handling(self) -> str:
        """What the process answering this address was started with. It is read
        from the engine's own properties: a query cannot tell, because how a NULL
        was stored is decided when a segment is written."""
        key = 'druid.generic.useDefaultValueForNull'
        try:
            value = (self._call('/status/properties') or {}).get(key)
        except EngineError:
            return 'unknown (the properties of the process cannot be read)'
        if value is None:
            return 'not set (the default of this version applies: legacy before Druid 28)'
        return '%s (%s=%s)' % ('legacy' if value == 'true' else 'sql-compatible', key, value)

    def run(self, compiled) -> List[List[Any]]:
        # The session properties of a Druid result are its query context.
        names, records = self.sql(compiled.sql, compiled.parameters, compiled.session_properties)
        return arranged(compiled, names, records)

    def existing(self) -> Set[str]:
        _, records = self.sql("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = 'druid'")
        return {record[0] for record in records}

    def _segments(self, name: str):
        _, rows = self.sql('SELECT "version", is_available, is_overshadowed FROM sys.segments WHERE datasource = ?',
                           [name])
        return rows

    def load(self, table, attempts: int = 300) -> None:
        """A SQL ingestion task. It is loaded when the task succeeded, a segment
        version that did not exist before is served, no older one is, and SQL
        knows every column: the broker learns a datasource a moment after its
        segment is served."""
        name = self.engine.table(table['name'])
        before = {row[0] for row in self._segments(name)}
        task = self._call('/druid/v2/sql/task', {'query': warehouse.druid(table, name)})
        identifier = (task or {}).get('taskId')
        if not identifier:
            raise EngineError('Druid accepted no ingestion task; is the multi-stage query extension loaded?')
        for attempt in range(attempts):
            status = self._call('/druid/indexer/v1/task/%s/status' % identifier)['status']['status']
            if status == 'SUCCESS':
                break
            if status == 'FAILED' or attempt == attempts - 1:
                raise EngineError('The ingestion task of %s %s' % (name, 'failed' if status == 'FAILED' else 'did not finish'))
            self.pause(1)
        written = {'__time'} | {c['name'] for c in table['columns'] if c['name'] != table.get('time')}
        for attempt in range(attempts):
            rows = self._segments(name)
            served = any(row[0] not in before and row[1] == 1 for row in rows)
            stale = any(row[0] in before and row[1] == 1 and row[2] == 0 for row in rows)
            if served and not stale:
                _, known = self.sql("SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS "
                                    "WHERE TABLE_SCHEMA = 'druid' AND TABLE_NAME = ?", [name])
                if written <= {row[0] for row in known}:
                    return
            self.pause(1)
        raise EngineError('The segments of %s were not served in time' % name)

    def drop(self, name: str) -> None:
        # Marks the datasource unused; its segments are removed by Druid's own kill tasks.
        self._call('/druid/coordinator/v1/datasources/' + name, method='DELETE')


EXECUTORS = {'presto': Presto, 'doris': Doris, 'druid': Druid}


def statements(engine: Engine, tables) -> List[str]:
    """What `load` runs on the engine, as text. On Druid each is submitted as a
    SQL ingestion task, to `/druid/v2/sql/task`."""
    if engine.dialect == 'druid':
        return [warehouse.druid(table, engine.table(table['name'])) for table in tables]
    out = ['CREATE %s IF NOT EXISTS %s' % ('SCHEMA' if engine.dialect == 'presto' else 'DATABASE', engine.schema)]
    for table in tables:
        name = engine.table(table['name'])
        if engine.dialect == 'presto':
            out += warehouse.presto(table, engine.schema, name)
        else:
            out += warehouse.doris(table, engine.schema, name, int(engine.option('replication_num', '1')))
    return out


def connect(engine: Engine):
    return EXECUTORS[engine.dialect](engine)
