"""Offline HTTP protocol tests; these are not real-engine T2 evidence."""
import importlib.util
from pathlib import Path
import unittest
import json
import threading
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
    def test_pages_and_busy_retry_execute_exact_golden(self):
        pages = [(503, {}), (200, {'nextUri': 'ORIGIN/v1/statement/next'}),
                 (200, dict(result([['web', 30]]), nextUri='ORIGIN/v1/statement/last')),
                 (200, result([['store', 7]]))]
        with server(pages) as (origin, requests):
            presto.verify(origin)
        self.assertEqual([r[0] for r in requests], ['POST', 'POST', 'GET', 'GET'])
        self.assertEqual(requests[0][2].decode(), presto.snapshot_sql(presto.SNAPSHOT.read_text()))
        self.assertEqual(requests[0][3]['X-Presto-Catalog'], 'memory')
        self.assertEqual(requests[0][3]['X-Presto-Time-Zone'], 'UTC')
        self.assertEqual(pages, [])

    def test_wrong_rows_columns_and_missing_results_fail(self):
        for payload in [result([['web', 31], ['store', 7]]),
                        {'columns': [{'name': 'wrong'}], 'data': [['web', 30], ['store', 7]]},
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
        for status in [302, 500]:
            with self.subTest(status=status), server([(status, {})]) as (origin, _):
                with self.assertRaises((OSError, ValueError)):
                    presto.verify(origin)

    def test_deadline_and_invalid_endpoint_fail(self):
        with self.assertRaises(TimeoutError):
            presto.execute('http://127.0.0.1:1', 'SELECT 1', timeout=0)
        for endpoint in ['file:///tmp/test', 'http://localhost/path']:
            with self.assertRaises(ValueError):
                presto.execute(endpoint, 'SELECT 1')


if __name__ == '__main__':
    unittest.main()
