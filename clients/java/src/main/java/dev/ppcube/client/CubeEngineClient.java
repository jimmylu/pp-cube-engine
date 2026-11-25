package dev.ppcube.client;

import dev.ppcube.engine.v1.CompileFailure;
import dev.ppcube.engine.v1.CompileRequest;
import dev.ppcube.engine.v1.CompileResponse;
import dev.ppcube.engine.v1.CubeEngineGrpc;
import dev.ppcube.engine.v1.GetMetaRequest;
import dev.ppcube.engine.v1.GetMetaResponse;
import io.grpc.CallOptions;
import io.grpc.Channel;
import io.grpc.ClientCall;
import io.grpc.ClientInterceptor;
import io.grpc.ForwardingClientCall.SimpleForwardingClientCall;
import io.grpc.Grpc;
import io.grpc.InsecureChannelCredentials;
import io.grpc.ManagedChannel;
import io.grpc.Metadata;
import io.grpc.MethodDescriptor;
import io.grpc.TlsChannelCredentials;
import java.io.IOException;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Objects;
import java.util.concurrent.TimeUnit;

/**
 * A client of the pp-cube-engine compile service.
 *
 * <p>It is thread-safe and holds one connection; make one and share it. A transport failure,
 * an unknown model version or a refused credential is a {@link io.grpc.StatusRuntimeException}
 * with its status code; a request the service answered without SQL is a
 * {@link CompileRefusedException}.
 */
public final class CubeEngineClient implements AutoCloseable {
  private static final Metadata.Key<String> AUTHORIZATION =
      Metadata.Key.of("authorization", Metadata.ASCII_STRING_MARSHALLER);

  private final ManagedChannel owned;
  private final CubeEngineGrpc.CubeEngineBlockingStub stub;
  private final Duration deadline;

  private CubeEngineClient(ManagedChannel owned, Channel channel, String token, Duration deadline) {
    this.owned = owned;
    CubeEngineGrpc.CubeEngineBlockingStub plain = CubeEngineGrpc.newBlockingStub(channel);
    this.stub = token == null ? plain : plain.withInterceptors(new Bearer(token));
    this.deadline = deadline;
  }

  /** A client of the service at {@code target}, such as {@code pp-cube-engine.internal:50051}. */
  public static Builder to(String target) {
    return new Builder(Objects.requireNonNull(target, "target"));
  }

  /**
   * Compiles a query to SQL.
   *
   * @throws CompileRefusedException when the service answers without SQL
   * @throws io.grpc.StatusRuntimeException NOT_FOUND for an unknown model version,
   *     UNAUTHENTICATED for a missing or wrong token, RESOURCE_EXHAUSTED when the service is
   *     full, DEADLINE_EXCEEDED, UNAVAILABLE
   */
  public CompiledQuery compile(Compilation compilation) {
    CompileRequest.Builder request =
        CompileRequest.newBuilder()
            .setModelVersion(compilation.modelVersion())
            .setQueryJson(compilation.queryJson())
            .setDialect(compilation.dialect())
            .setTimezone(compilation.timezone())
            .addAllSecurityFiltersJson(compilation.securityFilters())
            .setRequestId(compilation.requestId());
    CompileResponse response = timed().compile(request.build());
    switch (response.getResultCase()) {
      case COMPILED:
        return new CompiledQuery(response.getCompiled(), response.getIdentity());
      case FAILURE:
        CompileFailure failure = response.getFailure();
        throw new CompileRefusedException(kind(failure), failure.getMessage());
      default:
        throw new IllegalStateException("The service returned neither SQL nor a failure");
    }
  }

  /** The public cubes, views and members of a model version; never SQL or tables. */
  public GetMetaResponse meta(String modelVersion) {
    return timed().getMeta(GetMetaRequest.newBuilder().setModelVersion(modelVersion).build());
  }

  private CubeEngineGrpc.CubeEngineBlockingStub timed() {
    return stub.withDeadlineAfter(deadline.toMillis(), TimeUnit.MILLISECONDS);
  }

  private static CompileRefusedException.Kind kind(CompileFailure failure) {
    switch (failure.getKind()) {
      case FAILURE_KIND_MODEL:
        return CompileRefusedException.Kind.MODEL;
      case FAILURE_KIND_QUERY:
        return CompileRefusedException.Kind.QUERY;
      case FAILURE_KIND_UNSUPPORTED:
        return CompileRefusedException.Kind.UNSUPPORTED;
      default:
        // A kind this client does not know is not the caller's to fix.
        return CompileRefusedException.Kind.INTERNAL;
    }
  }

  /** Closes the connection, waiting briefly for calls in flight. */
  @Override
  public void close() {
    if (owned == null) {
      return;
    }
    owned.shutdown();
    try {
      if (!owned.awaitTermination(5, TimeUnit.SECONDS)) {
        owned.shutdownNow();
      }
    } catch (InterruptedException interrupted) {
      owned.shutdownNow();
      Thread.currentThread().interrupt();
    }
  }

  /** Builds a {@link CubeEngineClient}. */
  public static final class Builder {
    private final String target;
    private Path trusted;
    private boolean plaintext;
    private String token;
    private Duration deadline = Duration.ofSeconds(15);

    private Builder(String target) {
      this.target = target;
    }

    /**
     * Trust the certificates in this PEM file instead of the system's. For a service whose
     * certificate comes from a private authority.
     */
    public Builder trust(Path certificates) {
      this.trusted = Objects.requireNonNull(certificates, "certificates");
      return this;
    }

    /**
     * No encryption. The service allows it on loopback only, where it also needs no token; it
     * is for a service on the same host.
     */
    public Builder plaintext() {
      this.plaintext = true;
      return this;
    }

    /** The read token, sent as {@code authorization: Bearer}. */
    public Builder token(String token) {
      this.token = Objects.requireNonNull(token, "token").trim();
      return this;
    }

    /** How long one call may take. The service gives up a compilation after ten seconds. */
    public Builder deadline(Duration deadline) {
      this.deadline = Objects.requireNonNull(deadline, "deadline");
      return this;
    }

    public CubeEngineClient build() throws IOException {
      if (plaintext && token != null) {
        // A token over an unencrypted connection is a token given away.
        throw new IllegalStateException("A token is not sent over a plaintext connection");
      }
      ManagedChannel channel;
      if (plaintext) {
        channel = Grpc.newChannelBuilder(target, InsecureChannelCredentials.create()).build();
      } else if (trusted != null) {
        channel =
            Grpc.newChannelBuilder(
                    target,
                    TlsChannelCredentials.newBuilder().trustManager(trusted.toFile()).build())
                .build();
      } else {
        channel = Grpc.newChannelBuilder(target, TlsChannelCredentials.create()).build();
      }
      return new CubeEngineClient(channel, channel, token, deadline);
    }

    /** For tests and for hosts that manage channels themselves; the channel is not closed. */
    public CubeEngineClient over(Channel channel) {
      return new CubeEngineClient(null, channel, token, deadline);
    }
  }

  private static final class Bearer implements ClientInterceptor {
    private final String header;

    Bearer(String token) {
      this.header = "Bearer " + token;
    }

    @Override
    public <RequestT, ResponseT> ClientCall<RequestT, ResponseT> interceptCall(
        MethodDescriptor<RequestT, ResponseT> method, CallOptions options, Channel next) {
      return new SimpleForwardingClientCall<RequestT, ResponseT>(next.newCall(method, options)) {
        @Override
        public void start(Listener<ResponseT> listener, Metadata headers) {
          headers.put(AUTHORIZATION, header);
          super.start(listener, headers);
        }
      };
    }
  }
}
