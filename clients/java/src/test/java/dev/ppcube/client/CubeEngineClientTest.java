package dev.ppcube.client;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.google.protobuf.ByteString;
import dev.ppcube.engine.v1.CompileFailure;
import dev.ppcube.engine.v1.CompileRequest;
import dev.ppcube.engine.v1.CompileResponse;
import dev.ppcube.engine.v1.Compiled;
import dev.ppcube.engine.v1.CubeEngineGrpc;
import dev.ppcube.engine.v1.FailureKind;
import dev.ppcube.engine.v1.Identity;
import dev.ppcube.engine.v1.Null;
import dev.ppcube.engine.v1.OutputColumn;
import dev.ppcube.engine.v1.Parameter;
import dev.ppcube.engine.v1.SessionProperty;
import io.grpc.ManagedChannel;
import io.grpc.Metadata;
import io.grpc.Server;
import io.grpc.ServerCall;
import io.grpc.ServerCallHandler;
import io.grpc.ServerInterceptor;
import io.grpc.Status;
import io.grpc.StatusRuntimeException;
import io.grpc.inprocess.InProcessChannelBuilder;
import io.grpc.inprocess.InProcessServerBuilder;
import io.grpc.stub.StreamObserver;
import java.lang.reflect.Proxy;
import java.sql.PreparedStatement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/** The client against a stand-in service: what it sends, and how it reads each answer. */
class CubeEngineClientTest {
  private static final Metadata.Key<String> AUTHORIZATION =
      Metadata.Key.of("authorization", Metadata.ASCII_STRING_MARSHALLER);

  private final AtomicReference<CompileRequest> received = new AtomicReference<>();
  private final AtomicReference<String> authorization = new AtomicReference<>();
  private final AtomicReference<CompileResponse> answer = new AtomicReference<>();
  private Server server;
  private ManagedChannel channel;

  @BeforeEach
  void start() throws Exception {
    String name = InProcessServerBuilder.generateName();
    CubeEngineGrpc.CubeEngineImplBase service =
        new CubeEngineGrpc.CubeEngineImplBase() {
          @Override
          public void compile(CompileRequest request, StreamObserver<CompileResponse> observer) {
            received.set(request);
            if (request.getModelVersion().equals("missing")) {
              observer.onError(Status.NOT_FOUND.asRuntimeException());
              return;
            }
            observer.onNext(answer.get());
            observer.onCompleted();
          }
        };
    ServerInterceptor headers =
        new ServerInterceptor() {
          @Override
          public <RequestT, ResponseT> ServerCall.Listener<RequestT> interceptCall(
              ServerCall<RequestT, ResponseT> call,
              Metadata metadata,
              ServerCallHandler<RequestT, ResponseT> next) {
            authorization.set(metadata.get(AUTHORIZATION));
            return next.startCall(call, metadata);
          }
        };
    server =
        InProcessServerBuilder.forName(name)
            .directExecutor()
            .addService(service)
            .intercept(headers)
            .build()
            .start();
    channel = InProcessChannelBuilder.forName(name).directExecutor().build();
  }

  @AfterEach
  void stop() {
    channel.shutdownNow();
    server.shutdownNow();
  }

  private static Compiled compiled() {
    return Compiled.newBuilder()
        .setSql("SELECT sum(a) \"m\" FROM t WHERE c = ? AND d > ? AND e = ? AND f = ?")
        .addParams(Parameter.newBuilder().setText("East"))
        .addParams(Parameter.newBuilder().setNumber(1000))
        .addParams(Parameter.newBuilder().setBoolean(true))
        .addParams(Parameter.newBuilder().setNull(Null.getDefaultInstance()))
        .addColumns(
            OutputColumn.newBuilder()
                .setAlias("policies__issued_at_month")
                .setMember("policies.issued_at")
                .setType("time")
                .setGranularity("month"))
        .addColumns(
            OutputColumn.newBuilder()
                .setAlias("m")
                .setMember("policies.written_premium")
                .setType("number"))
        .addSessionProperties(
            SessionProperty.newBuilder().setName("offset_clause_enabled").setValue("true"))
        .setFingerprint(ByteString.copyFrom(new byte[] {0x00, 0x0f, (byte) 0xa5}))
        .build();
  }

  private static Identity identity() {
    return Identity.newBuilder()
        .setKernelVersion("0.2.0")
        .setModelVersion("property/2026-11")
        .setModelDigest("sha256:abc")
        .build();
  }

  @Test
  void aCompilationCarriesTheQuestionTheRulesAndTheToken() {
    answer.set(CompileResponse.newBuilder().setCompiled(compiled()).setIdentity(identity()).build());
    CubeEngineClient client = CubeEngineClient.to("ignored").token(" reader-token-0123456789\n").over(channel);

    CompiledQuery query =
        client.compile(
            Compilation.of("property/2026-11", "presto", "{\"measures\":[\"policies.written_premium\"]}")
                .timezone("Asia/Tokyo")
                .securityFilter("{\"member\":\"branches.region_name\",\"operator\":\"equals\",\"values\":[\"East\"]}")
                .requestId("r-1")
                .build());

    CompileRequest request = received.get();
    assertEquals("property/2026-11", request.getModelVersion());
    assertEquals("presto", request.getDialect());
    assertEquals("Asia/Tokyo", request.getTimezone());
    assertEquals("r-1", request.getRequestId());
    assertEquals(1, request.getSecurityFiltersJsonCount());
    // The token is trimmed, as the service trims the file it reads its own from.
    assertEquals("Bearer reader-token-0123456789", authorization.get());

    assertEquals(4, query.parameters().size());
    assertEquals(Arrays.asList("East", 1000.0, true, null), query.parameters());
    assertEquals("true", query.sessionProperties().get("offset_clause_enabled"));
    assertEquals("000fa5", query.fingerprint());
    assertEquals("0.2.0", query.kernelVersion());
    assertEquals("sha256:abc", query.modelDigest());
    assertEquals("month", query.columns().get(0).granularity());
    assertNull(query.columns().get(1).granularity());
    assertEquals("policies.written_premium", query.columns().get(1).member());
  }

  @Test
  void parametersAreBoundInOrderByTheirType() throws Exception {
    answer.set(CompileResponse.newBuilder().setCompiled(compiled()).setIdentity(identity()).build());
    CompiledQuery query =
        CubeEngineClient.to("ignored").over(channel).compile(Compilation.of("v", "presto", "{}").build());
    List<String> bound = new ArrayList<>();
    PreparedStatement statement =
        (PreparedStatement)
            Proxy.newProxyInstance(
                getClass().getClassLoader(),
                new Class<?>[] {PreparedStatement.class},
                (proxy, method, arguments) -> {
                  bound.add(method.getName() + Arrays.toString(arguments));
                  return null;
                });

    query.bind(statement);

    assertEquals(
        Arrays.asList(
            "setString[1, East]",
            "setDouble[2, 1000.0]",
            "setBoolean[3, true]",
            "setNull[4, " + Types.NULL + "]"),
        bound);
    // No token was configured, so none is sent.
    assertNull(authorization.get());
  }

  @Test
  void anAnswerWithoutSqlIsARefusalOfItsKind() {
    CubeEngineClient client = CubeEngineClient.to("ignored").over(channel);
    Object[][] kinds = {
      {FailureKind.FAILURE_KIND_MODEL, CompileRefusedException.Kind.MODEL},
      {FailureKind.FAILURE_KIND_QUERY, CompileRefusedException.Kind.QUERY},
      {FailureKind.FAILURE_KIND_UNSUPPORTED, CompileRefusedException.Kind.UNSUPPORTED},
      {FailureKind.FAILURE_KIND_INTERNAL, CompileRefusedException.Kind.INTERNAL},
      // A kind added to the contract later is not the caller's to fix.
      {FailureKind.FAILURE_KIND_UNSPECIFIED, CompileRefusedException.Kind.INTERNAL},
    };
    for (Object[] kind : kinds) {
      answer.set(
          CompileResponse.newBuilder()
              .setFailure(
                  CompileFailure.newBuilder()
                      .setKind((FailureKind) kind[0])
                      .setMessage("Measure `policies.missing` is not in the model"))
              .build());
      CompileRefusedException refused =
          assertThrows(
              CompileRefusedException.class,
              () -> client.compile(Compilation.of("v", "presto", "{}").build()));
      assertEquals(kind[1], refused.kind());
      assertEquals("Measure `policies.missing` is not in the model", refused.getMessage());
    }
  }

  @Test
  void aFailureOfTheCallKeepsItsStatus() {
    CubeEngineClient client = CubeEngineClient.to("ignored").over(channel);
    StatusRuntimeException failed =
        assertThrows(
            StatusRuntimeException.class,
            () -> client.compile(Compilation.of("missing", "presto", "{}").build()));
    assertEquals(Status.Code.NOT_FOUND, failed.getStatus().getCode());
    // An empty answer is not taken for SQL.
    answer.set(CompileResponse.getDefaultInstance());
    assertThrows(
        IllegalStateException.class,
        () -> client.compile(Compilation.of("v", "presto", "{}").build()));
  }

  @Test
  void aTokenIsNeverSentUnencrypted() {
    assertThrows(
        IllegalStateException.class,
        () -> CubeEngineClient.to("localhost:1").plaintext().token("reader-token-0123456789").build());
  }
}
