package dev.cindervale.client;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * One question put to the kernel: a Cube query against a model version, for a dialect.
 *
 * <p>The query and the security filters are JSON text in Cube's own format, as the
 * caller already holds them; this client does not parse them.
 */
public final class Compilation {
  private final String modelVersion;
  private final String queryJson;
  private final String dialect;
  private final String timezone;
  private final List<String> securityFilters;
  private final String requestId;

  private Compilation(Builder builder) {
    this.modelVersion = Objects.requireNonNull(builder.modelVersion, "modelVersion");
    this.queryJson = Objects.requireNonNull(builder.queryJson, "queryJson");
    this.dialect = Objects.requireNonNull(builder.dialect, "dialect");
    this.timezone = builder.timezone;
    this.securityFilters = Collections.unmodifiableList(new ArrayList<>(builder.securityFilters));
    this.requestId = builder.requestId;
  }

  /** A compilation of {@code queryJson} against {@code modelVersion} for {@code dialect}. */
  public static Builder of(String modelVersion, String dialect, String queryJson) {
    Builder builder = new Builder();
    builder.modelVersion = modelVersion;
    builder.dialect = dialect;
    builder.queryJson = queryJson;
    return builder;
  }

  public String modelVersion() {
    return modelVersion;
  }

  public String queryJson() {
    return queryJson;
  }

  public String dialect() {
    return dialect;
  }

  public String timezone() {
    return timezone;
  }

  public List<String> securityFilters() {
    return securityFilters;
  }

  public String requestId() {
    return requestId;
  }

  /** Builds a {@link Compilation}. */
  public static final class Builder {
    private String modelVersion;
    private String queryJson;
    private String dialect;
    private String timezone = "UTC";
    private final List<String> securityFilters = new ArrayList<>();
    private String requestId = "";

    private Builder() {}

    /** The timezone of the query's dates when the query names none. UTC by default. */
    public Builder timezone(String timezone) {
      this.timezone = Objects.requireNonNull(timezone, "timezone");
      return this;
    }

    /**
     * One row-level rule, a Cube filter as JSON. The kernel applies every one on top of the
     * query, where the query cannot widen it. Only the system that enforces access may choose
     * them: never take them from an end user.
     */
    public Builder securityFilter(String filterJson) {
      this.securityFilters.add(Objects.requireNonNull(filterJson, "filterJson"));
      return this;
    }

    /** An id of the caller's own, returned in the service's log; not part of the fingerprint. */
    public Builder requestId(String requestId) {
      this.requestId = Objects.requireNonNull(requestId, "requestId");
      return this;
    }

    public Compilation build() {
      return new Compilation(this);
    }
  }
}
