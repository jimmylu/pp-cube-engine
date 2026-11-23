"""Offline HTTP protocol tests; these are not real-engine T2 evidence."""
import importlib.util
from pathlib import Path
import unittest
import json
import threading
import time
import http.client
import io
from unittest.mock import patch
from contextlib import redirect_stderr
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

SPEC = importlib.util.spec_from_file_location(
    'presto_engine', Path(__file__).parents[1] / 'engines/presto.py')
presto = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(presto)


@contextmanager
def server(pages):
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def respond(self):
            body = self.rfile.read(int(self.headers.get('Content-Length', 0)))
            requests.append((self.command, self.path, body, self.headers))
            status, payload = pages.pop(0)
            payload = json.dumps(payload).replace('ORIGIN', origin).encode()
            self.send_response(status)
            self.send_header('Content-Length', str(len(payload)))
            if status == 302:
                self.send_header('Location', origin + '/redirected')
            self.end_headers()
            self.wfile.write(payload)

        do_POST = respond
        do_GET = respond
        do_DELETE = respond

    httpd = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    origin = 'http://127.0.0.1:' + str(httpd.server_port)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    try:
        yield origin, requests
    finally:
        httpd.shutdown()
        thread.join()
        httpd.server_close()


COLUMNS = ['sales__channel_cd', 'sales__amount']


def result(rows, columns=COLUMNS):
    return {'columns': [{'name': name} for name in columns], 'data': rows}


def case(**changes):
    value = {'name': 'by_channel', 'scenario': 'B0-PRESTO-003', 'fixture': ['DROP X', 'CREATE X'],
             'sql': 'SELECT ? AS a', 'params': [{'type': 'text', 'value': 'C1'}],
             'columns': COLUMNS, 'rows': [['C1', 30]]}
    value.update(changes)
    return value


class ManifestTests(unittest.TestCase):
    def test_cases_share_fixtures_and_send_session_and_typed_parameters(self):
        cases = [case(), case(name='again', rows=[['C1', 30.0]]),
                 case(name='day', fixture=['CREATE Y'], columns=['day', 'amount'],
                      params=[{'type': 'text', 'value': "a'b"}, {'type': 'number', 'value': 3},
                              {'type': 'bool', 'value': True}, {'type': 'null'}],
                      rows=[[{'timestamp': '2026-10-01T00:00:00'}, 20]])]
        pages = [(200, {}), (200, {}), (200, result([['C1', 30]])), (200, result([['C1', 30]])), (200, {}),
                 (200, result([['2026-10-01 00:00:00.000', 20]], ['day', 'amount']))]
        with server(pages) as (origin, requests):
            presto.verify_manifest(origin, cases, {'offset_clause_enabled': 'true'})
        # One fixture load per distinct fixture, in order, then each case.
        self.assertEqual([r[2].decode() for r in requests],
                         ['DROP X', 'CREATE X', "EXECUTE cube_case USING 'C1'", "EXECUTE cube_case USING 'C1'",
                          'CREATE Y', "EXECUTE cube_case USING 'a''b', DOUBLE '3.0', TRUE, NULL"])
        for request in requests:
            self.assertEqual(request[3]['X-Presto-Session'], 'offset_clause_enabled=true')
        header = requests[2][3]['X-Presto-Prepared-Statement']
        self.assertEqual(presto.urllib.parse.unquote_plus(header.split('=', 1)[1]), 'SELECT ? AS a')
        self.assertIsNone(requests[0][3]['X-Presto-Prepared-Statement'])

    def test_wrong_rows_columns_buckets_and_missing_results_fail(self):
        day = case(columns=['day', 'amount'], rows=[[{'timestamp': '2026-10-01T00:00:00'}, 20]])
        for expected, payload in [
                (case(), result([['C1', 31]])), (case(), result([['C1', 30], ['C2', 7]])),
                (case(), result([['C1', 30]], ['wrong', 'sales__amount'])), (case(), {}),
                (case(rows=[]), result([['C1', 30]])),
                (day, result([['2026-09-30 00:00:00.000', 20]], ['day', 'amount'])),
                (day, result([['2026-10-01 00:00:00.000', 20], ['2026-11-01 00:00:00.000', 0]], ['day', 'amount']))]:
            with self.subTest(payload=payload), server([(200, {}), (200, {}), (200, payload)]) as (origin, _):
                with self.assertRaises(ValueError) as failure:
                    presto.verify_manifest(origin, [expected])
                self.assertEqual(presto.failure_category(failure.exception),
                                 'result differs from the case expectation')

    def test_an_empty_expected_page_passes_only_with_no_rows(self):
        with server([(200, {}), (200, {}), (200, result([]))]) as (origin, _):
            presto.verify_manifest(origin, [case(rows=[])])

    def test_unknown_parameters_and_cells_fail_closed(self):
        for item in [{'type': 'date', 'value': 'x'}, {'type': 'text', 'value': 1}, {'type': 'text'},
                     {'type': 'text', 'value': 'a\x00'}, {'type': 'number', 'value': True},
                     {'type': 'number', 'value': float('inf')}, {'type': 'null', 'value': 1},
                     {'type': 'text', 'value': 'a', 'extra': 1}]:
            with self.subTest(item=item), self.assertRaises(ValueError) as failure:
                presto.literal(item)
            self.assertEqual(presto.failure_category(failure.exception), 'unsupported manifest parameter')
        for value in [{'date': '2026-10-01'}, {'timestamp': '2026-10-01'}, {'timestamp': 1}, [1]]:
            with self.subTest(value=value), self.assertRaises(ValueError) as failure:
                presto.cell(value)
            self.assertEqual(presto.failure_category(failure.exception), 'unsupported expected cell')
        self.assertIsNone(presto.cell(None))


class ProtocolTests(unittest.TestCase):
    def test_text_binding_uses_prepared_header_without_replacing_placeholders(self):
        sql = "SELECT ? AS value"
        value = "C1' OR 'x'='x; -- café"
        with server([(200, result([[value, 30]]))]) as (origin, requests):
            presto.execute(origin, sql, parameters=({'type': 'text', 'value': value},))
        self.assertEqual(requests[0][2].decode(),
                         "EXECUTE cube_case USING 'C1'' OR ''x''=''x; -- café'")
        header = requests[0][3]['X-Presto-Prepared-Statement']
        self.assertEqual(presto.urllib.parse.unquote_plus(header.split('=', 1)[1]), sql)
        self.assertIsNone(requests[0][3]['X-Presto-Session'])

    def test_pages_and_busy_retry_collect_every_row(self):
        pages = [(503, {}), (200, {'nextUri': 'ORIGIN/v1/statement/next'}),
                 (200, dict(result([['C1', 30]]), nextUri='ORIGIN/v1/statement/last')),
                 (200, result([['C2', 7]]))]
        with server(pages) as (origin, requests):
            self.assertEqual(presto.execute(origin, 'SELECT 1'), (COLUMNS, [['C1', 30], ['C2', 7]]))
        self.assertEqual([r[0] for r in requests], ['POST', 'POST', 'GET', 'GET'])
        self.assertEqual(requests[0][2].decode(), 'SELECT 1')
        self.assertEqual(requests[0][3]['X-Presto-Catalog'], 'memory')
        self.assertEqual(requests[0][3]['X-Presto-Time-Zone'], 'UTC')
        self.assertEqual(pages, [])

    def test_query_error_cancels_pending_query(self):
        with server([(200, {'error': {'message': 'bad'}, 'nextUri': 'ORIGIN/next'}),
                     (200, {})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'query error'):
                presto.execute(origin, 'SELECT 1')
        self.assertEqual([r[0] for r in requests], ['POST', 'DELETE'])

    def test_cross_origin_next_uri_is_never_followed(self):
        with server([(200, {'nextUri': 'http://127.0.0.1:1/next'})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'configured origin'):
                presto.execute(origin, 'SELECT 1')
        self.assertEqual(len(requests), 1)

    def test_no_redirect_or_http_error_is_success(self):
        with server([(302, {})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'redirects are not supported'):
                presto.execute(origin, 'SELECT 1')
        self.assertEqual(len(requests), 1)
        with server([(500, {})]) as (origin, _):
            with self.assertRaises(OSError):
                presto.execute(origin, 'SELECT 1')

    def test_slow_body_is_interrupted_by_wall_deadline(self):
        class SlowHandler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_POST(self):
                self.send_response(200)
                payload = b'{"padding":"' + b'x' * 50 + b'"}'
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                try:
                    for byte in payload:
                        self.wfile.write(bytes([byte]))
                        self.wfile.flush()
                        time.sleep(0.02)
                except OSError:
                    pass

        httpd = ThreadingHTTPServer(('127.0.0.1', 0), SlowHandler)
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        try:
            started = time.monotonic()
            with self.assertRaisesRegex(TimeoutError, 'Presto query deadline'):
                presto.execute('http://127.0.0.1:' + str(httpd.server_port), 'SELECT 1', 0.2)
            self.assertLess(time.monotonic() - started, 0.8)
        finally:
            httpd.shutdown()
            thread.join()
            httpd.server_close()

    def test_http_protocol_errors_are_sanitized(self):
        for error in [http.client.BadStatusLine('private-response'),
                      http.client.IncompleteRead(b'private-response')]:
            output = io.StringIO()
            with patch.object(presto.MANIFEST, 'compiled_manifest', return_value=([case()], {})), \
                    patch.object(presto, 'verify_manifest', side_effect=error), \
                    patch('sys.argv', ['presto.py', '--endpoint', 'http://localhost']), \
                    redirect_stderr(output):
                self.assertEqual(presto.main(), 1)
            self.assertNotIn('private-response', output.getvalue())
            self.assertIn('FAILED', output.getvalue())

    def test_cancel_protocol_failure_preserves_query_error(self):
        class Response:
            def __enter__(self):
                return io.StringIO(json.dumps({'error': {'message': 'bad'},
                                              'nextUri': 'http://localhost/next'}))

            def __exit__(self, *args):
                pass

        with patch.object(presto.urllib.request, 'build_opener') as opener:
            opener.return_value.open.side_effect = [Response(), http.client.BadStatusLine('bad')]
            with self.assertRaisesRegex(ValueError, 'query error'):
                presto.execute('http://localhost', 'SELECT 1')

    def test_deadline_and_invalid_endpoint_fail(self):
        with self.assertRaises(TimeoutError):
            presto.execute('http://127.0.0.1:1', 'SELECT 1', timeout=0)
        for endpoint in ['file:///tmp/test', 'http://localhost/path']:
            with self.assertRaises(ValueError):
                presto.execute(endpoint, 'SELECT 1')


if __name__ == '__main__':
    unittest.main()
