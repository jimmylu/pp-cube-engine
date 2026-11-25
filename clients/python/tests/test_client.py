"""The client against a stand-in service: what it sends, and how it reads each answer."""
from concurrent import futures
import unittest

import grpc

from ppcube import Client, CompiledQuery, CompileRefused
from ppcube._contract import engine_pb2, engine_pb2_grpc

SQL = 'SELECT sum(a) "m" FROM t WHERE c = ? AND d > ? AND e = ? AND f = ?'


def compiled():
    return engine_pb2.Compiled(
        sql=SQL,
        params=[engine_pb2.Parameter(text='East'), engine_pb2.Parameter(number=1000),
                engine_pb2.Parameter(boolean=True), engine_pb2.Parameter(null=engine_pb2.Null())],
        columns=[engine_pb2.OutputColumn(alias='policies__issued_at_month', member='policies.issued_at',
                                         type='time', granularity='month'),
                 engine_pb2.OutputColumn(alias='m', member='policies.written_premium', type='number')],
        session_properties=[engine_pb2.SessionProperty(name='offset_clause_enabled', value='true')],
        fingerprint=bytes([0x00, 0x0f, 0xa5]))


class StandIn(engine_pb2_grpc.CubeEngineServicer):
    def __init__(self):
        self.request = None
        self.authorization = None
        self.answer = None

    def Compile(self, request, context):
        self.request = request
        self.authorization = dict(context.invocation_metadata()).get('authorization')
        if request.model_version == 'missing':
            context.abort(grpc.StatusCode.NOT_FOUND, 'no such version')
        return self.answer


class ClientTests(unittest.TestCase):
    def setUp(self):
        self.service = StandIn()
        self.server = grpc.server(futures.ThreadPoolExecutor(max_workers=2))
        engine_pb2_grpc.add_CubeEngineServicer_to_server(self.service, self.server)
        port = self.server.add_insecure_port('127.0.0.1:0')
        self.server.start()
        self.addCleanup(self.server.stop, None)
        self.channel = grpc.insecure_channel('127.0.0.1:%d' % port)
        self.addCleanup(self.channel.close)
        self.identity = engine_pb2.Identity(kernel_version='0.2.0', model_version='property/2026-11',
                                            model_digest='sha256:abc')

    def test_a_compilation_carries_the_question_the_rules_and_the_token(self):
        self.service.answer = engine_pb2.CompileResponse(compiled=compiled(), identity=self.identity)
        rule = {'member': 'branches.region_name', 'operator': 'equals', 'values': ['East']}
        # The channel is the caller's here, so the token travels as it would over TLS.
        client = Client(channel=self.channel, token=' reader-token-0123456789\n')

        query = client.compile('property/2026-11', 'presto', {'measures': ['policies.written_premium']},
                               timezone='Asia/Tokyo', security_filters=[rule], request_id='r-1')

        request = self.service.request
        self.assertEqual(request.model_version, 'property/2026-11')
        self.assertEqual(request.dialect, 'presto')
        self.assertEqual(request.timezone, 'Asia/Tokyo')
        self.assertEqual(request.request_id, 'r-1')
        # A mapping is sent as JSON; text is sent as it is.
        self.assertEqual(request.query_json, '{"measures": ["policies.written_premium"]}')
        self.assertEqual(list(request.security_filters_json),
                         ['{"member": "branches.region_name", "operator": "equals", "values": ["East"]}'])
        # The token is trimmed, as the service trims the file it reads its own from.
        self.assertEqual(self.service.authorization, 'Bearer reader-token-0123456789')

        self.assertEqual(query.parameters, ('East', 1000.0, True, None))
        self.assertEqual(query.session_properties, {'offset_clause_enabled': 'true'})
        self.assertEqual(query.fingerprint, '000fa5')
        self.assertEqual((query.kernel_version, query.model_digest), ('0.2.0', 'sha256:abc'))
        self.assertEqual(query.columns[0].granularity, 'month')
        self.assertIsNone(query.columns[1].granularity)
        self.assertEqual(query.columns[1].member, 'policies.written_premium')

        client.compile('v', 'presto', '{"measures":[]}')
        self.assertEqual(self.service.request.query_json, '{"measures":[]}')
        self.assertEqual(self.service.request.timezone, 'UTC')

    def test_no_token_is_sent_unless_one_is_given(self):
        self.service.answer = engine_pb2.CompileResponse(compiled=compiled(), identity=self.identity)
        Client(channel=self.channel).compile('v', 'presto', {})
        self.assertIsNone(self.service.authorization)

    def test_an_answer_without_sql_is_a_refusal_of_its_kind(self):
        client = Client(channel=self.channel)
        for sent, kind in [(engine_pb2.FAILURE_KIND_MODEL, 'model'), (engine_pb2.FAILURE_KIND_QUERY, 'query'),
                           (engine_pb2.FAILURE_KIND_UNSUPPORTED, 'unsupported'),
                           (engine_pb2.FAILURE_KIND_INTERNAL, 'internal'),
                           # A kind added to the contract later is not the caller's to fix.
                           (engine_pb2.FAILURE_KIND_UNSPECIFIED, 'internal'), (99, 'internal')]:
            with self.subTest(sent=sent):
                self.service.answer = engine_pb2.CompileResponse(failure=engine_pb2.CompileFailure(
                    kind=sent, message='Measure `policies.missing` is not in the model'))
                with self.assertRaises(CompileRefused) as refused:
                    client.compile('v', 'presto', {})
                self.assertEqual(refused.exception.kind, kind)
                self.assertEqual(str(refused.exception), 'Measure `policies.missing` is not in the model')

    def test_a_failure_of_the_call_keeps_its_status(self):
        client = Client(channel=self.channel)
        with self.assertRaises(grpc.RpcError) as failed:
            client.compile('missing', 'presto', {})
        self.assertEqual(failed.exception.code(), grpc.StatusCode.NOT_FOUND)
        self.assertNotIsInstance(failed.exception, CompileRefused)
        # An empty answer is not taken for SQL.
        self.service.answer = engine_pb2.CompileResponse()
        with self.assertRaises(RuntimeError):
            client.compile('v', 'presto', {})

    def test_a_token_is_never_sent_unencrypted(self):
        with self.assertRaises(ValueError):
            Client('localhost:1', plaintext=True, token='reader-token-0123456789')
        with self.assertRaises(ValueError):
            Client()

    def test_a_client_closes_only_the_channel_it_made(self):
        with Client('127.0.0.1:1', plaintext=True) as owned:
            self.assertTrue(owned._owned)
        with Client(channel=self.channel) as borrowed:
            self.assertFalse(borrowed._owned)
        self.service.answer = engine_pb2.CompileResponse(compiled=compiled(), identity=self.identity)
        # The caller's channel still works after the client is closed.
        Client(channel=self.channel).compile('v', 'presto', {})


class ParamstyleTests(unittest.TestCase):
    def query(self, sql, parameters):
        return CompiledQuery(sql=sql, parameters=tuple(parameters), columns=(), session_properties={},
                             fingerprint='', kernel_version='', model_version='', model_digest='')

    def test_qmark_is_the_statement_as_it_is(self):
        self.assertEqual(self.query(SQL, range(4)).sql_as('qmark'), SQL)

    def test_format_and_numeric_replace_placeholders_and_nothing_else(self):
        # A literal with ?, % and a doubled quote; a quoted identifier with ?; a LIKE with %.
        sql = ("SELECT \"what?\" FROM t WHERE a = ? AND b = 'it''s 100% ?' "
               "AND c LIKE CONCAT('%', LOWER(?), '%') AND `d?` = ?")
        query = self.query(sql, ['x', 'y', 'z'])
        self.assertEqual(
            query.sql_as('format'),
            "SELECT \"what?\" FROM t WHERE a = %s AND b = 'it''s 100%% ?' "
            "AND c LIKE CONCAT('%%', LOWER(%s), '%%') AND `d?` = %s")
        self.assertEqual(
            query.sql_as('numeric'),
            "SELECT \"what?\" FROM t WHERE a = :1 AND b = 'it''s 100% ?' "
            "AND c LIKE CONCAT('%', LOWER(:2), '%') AND `d?` = :3")
        # What a driver of that style does with it gives the kernel's statement back.
        self.assertEqual(query.sql_as('format') % ('?', '?', '?'), sql)

    def test_a_statement_the_scan_cannot_follow_is_not_rewritten(self):
        for sql, parameters in [("SELECT ? FROM t WHERE a = 'open", ['x']), ('SELECT ?, ?', ['x']),
                                ('SELECT 1', ['x'])]:
            with self.subTest(sql=sql):
                with self.assertRaises(ValueError):
                    self.query(sql, parameters).sql_as('format')
        with self.assertRaises(ValueError):
            self.query('SELECT 1', []).sql_as('pyformat')


if __name__ == '__main__':
    unittest.main()
