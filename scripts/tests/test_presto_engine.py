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


class SnapshotTests(unittest.TestCase):
    def test_parameterized_snapshot_is_not_silently_executed(self):
        with self.assertRaisesRegex(ValueError, 'parameters'):
            presto.snapshot_sql('---\nsource: test\n---\nSELECT ?\n-- parameters: [1]\n')


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


def result(rows):
    return {'columns': [{'name': name} for name in presto.EXPECTED_COLUMNS], 'data': rows}


class ProtocolTests(unittest.TestCase):
    def test_suite_executes_both_real_golden_requests_and_checks_filtered_rows(self):
        pages = [(200, result(presto.EXPECTED_ROWS)), (200, result([['C1', 30]]))]
        with server(pages) as (origin, requests):
            presto.verify_suite(origin)
        self.assertEqual(len(requests), 2)
        sql, params = presto.text_filter_snapshot(presto.FILTER_SNAPSHOT.read_text())
        self.assertEqual(params, ('C1',))
        self.assertIn(' = ?', sql)
        self.assertEqual(requests[1][2], b"EXECUTE cube_smoke USING 'C1'")
        with server([(200, result(presto.EXPECTED_ROWS)),
                     (200, result([['C2', 7]]))]) as (origin, _):
            with self.assertRaisesRegex(ValueError, 'result differs'):
                presto.verify_suite(origin)

    def test_unsupported_parameter_shapes_fail_before_http(self):
        for value in ('[]', '[Int(1)]', '[Text("a"), Text("b")]', '[Text(bad)]'):
            with self.subTest(value=value), self.assertRaises(ValueError) as failure:
                presto.text_filter_snapshot('---\nsource: test\n---\nSELECT ?\n-- parameters: ' + value)
            self.assertEqual(presto.failure_category(failure.exception),
                             'expected one text snapshot parameter')

    def test_text_binding_uses_prepared_header_without_replacing_placeholders(self):
        sql = "SELECT ? AS value"
        value = "C1' OR 'x'='x; -- café"
        with server([(200, result([[value, 30]]))]) as (origin, requests):
            presto.execute(origin, sql, parameters=(value,))
        self.assertEqual(requests[0][2].decode(),
                         "EXECUTE cube_smoke USING 'C1'' OR ''x''=''x; -- café'")
        header = requests[0][3]['X-Presto-Prepared-Statement']
        self.assertEqual(presto.urllib.parse.unquote_plus(header.split('=', 1)[1]), sql)

    def test_pages_and_busy_retry_execute_exact_golden(self):
        pages = [(503, {}), (200, {'nextUri': 'ORIGIN/v1/statement/next'}),
                 (200, dict(result([['C1', 30]]), nextUri='ORIGIN/v1/statement/last')),
                 (200, result([['C2', 7]]))]
        with server(pages) as (origin, requests):
            presto.verify(origin)
        self.assertEqual([r[0] for r in requests], ['POST', 'POST', 'GET', 'GET'])
        self.assertEqual(requests[0][2].decode(), presto.snapshot_sql(presto.SNAPSHOT.read_text()))
        self.assertEqual(requests[0][3]['X-Presto-Catalog'], 'memory')
        self.assertEqual(requests[0][3]['X-Presto-Time-Zone'], 'UTC')
        self.assertEqual(pages, [])

    def test_wrong_rows_columns_and_missing_results_fail(self):
        for payload in [result([['C1', 31], ['C2', 7]]),
                        {'columns': [{'name': 'wrong'}], 'data': [['C1', 30], ['C2', 7]]},
                        {}]:
            with self.subTest(payload=payload), server([(200, payload)]) as (origin, _):
                with self.assertRaisesRegex(ValueError, 'result differs'):
                    presto.verify(origin)

    def test_query_error_cancels_pending_query(self):
        with server([(200, {'error': {'message': 'bad'}, 'nextUri': 'ORIGIN/next'}),
                     (200, {})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'query error'):
                presto.verify(origin)
        self.assertEqual([r[0] for r in requests], ['POST', 'DELETE'])

    def test_cross_origin_next_uri_is_never_followed(self):
        with server([(200, {'nextUri': 'http://127.0.0.1:1/next'})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'configured origin'):
                presto.verify(origin)
        self.assertEqual(len(requests), 1)

    def test_no_redirect_or_http_error_is_success(self):
        with server([(302, {})]) as (origin, requests):
            with self.assertRaisesRegex(ValueError, 'redirects are not supported'):
                presto.verify(origin)
        self.assertEqual(len(requests), 1)
        with server([(500, {})]) as (origin, _):
            with self.assertRaises(OSError):
                presto.verify(origin)

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
            with patch.object(presto, 'verify', side_effect=error), \
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
