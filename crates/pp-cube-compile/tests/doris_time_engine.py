"""Real local Docker verification of SQL freshly compiled by doris_time.rs."""
import datetime
import importlib.util
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('doris', root / 'scripts/engines/doris-container.py')
doris = importlib.util.module_from_spec(spec)
spec.loader.exec_module(doris)
cases = json.load(sys.stdin)


def suite(connection):
    cursor = connection.cursor()
    for sql in [
        'CREATE DATABASE dw',
        "CREATE TABLE dw.sales_dtl (sold_at DATETIMEV2(3), amount DOUBLE) DUPLICATE KEY(sold_at) DISTRIBUTED BY HASH(sold_at) BUCKETS 1 PROPERTIES ('replication_num'='1')",
        "INSERT INTO dw.sales_dtl VALUES ('2026-09-30 14:59:59.999',10),('2026-09-30 15:00:00.000',20),('2026-10-31 14:59:59.999',7),('2026-10-31 15:00:00.000',100)"
    ]:
        cursor.execute(sql)
    cursor.close()
    cursor = connection.cursor(prepared=True)
    expected = {
        'day': [('2026-10-01', 20), ('2026-10-31', 7)],
        'week': [('2026-09-28', 20), ('2026-10-26', 7)],
        'month': [('2026-10-01', 27)], 'quarter': [('2026-10-01', 27)],
        'year': [('2026-01-01', 27)],
        'dst': [('2026-11-01', 15)], 'security': [(27,)],
        'inDateRange': [(27,)], 'notInDateRange': [(110,)],
        'beforeDate': [(10,)], 'beforeOrOnDate': [(30,)],
        'afterDate': [(107,)], 'afterOrOnDate': [(127,)],
    }
    for case in cases:
        if case['name'] == 'dst':
            cursor.close()
            cursor = connection.cursor()
            cursor.execute("TRUNCATE TABLE dw.sales_dtl")
            cursor.execute("INSERT INTO dw.sales_dtl VALUES ('2026-11-01 04:00:00',3),('2026-11-01 05:30:00',5),('2026-11-01 06:30:00',7),('2026-11-02 05:00:00',100)")
            cursor.close()
            cursor = connection.cursor(prepared=True)
        if case['name'] == 'security':
            cursor.close()
            cursor = connection.cursor()
            cursor.execute("TRUNCATE TABLE dw.sales_dtl")
            cursor.execute("INSERT INTO dw.sales_dtl VALUES ('2026-09-30 14:59:59.999',10),('2026-09-30 15:00:00.000',20),('2026-10-31 14:59:59.999',7),('2026-10-31 15:00:00.000',100)")
            cursor.close()
            cursor = connection.cursor(prepared=True)
        name = case['name']
        rows = expected[name]
        columns = ['sales__amount']
        if name in ('day', 'week', 'month', 'quarter', 'year', 'dst'):
            columns = ['sales__sold_at_' + ('day' if name == 'dst' else name), 'sales__amount']
            rows = [(datetime.datetime.fromisoformat(day), amount) for day, amount in rows]
        doris.verify(cursor, case['sql'], tuple(case['params']), columns, rows)
    cursor.close()


doris.suite = suite
doris.run()
print('Doris live-compiled temporal execution: PASSED — 13 cases')
