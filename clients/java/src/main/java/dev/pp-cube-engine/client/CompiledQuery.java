package dev.cindervale.client;

import dev.cindervale.engine.v1.Compiled;
import dev.cindervale.engine.v1.Identity;
import dev.cindervale.engine.v1.OutputColumn;
import dev.cindervale.engine.v1.Parameter;
import dev.cindervale.engine.v1.SessionProperty;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * SQL the kernel compiled, with everything needed to run it as it is.
 *
 * <p>Three rules for whoever executes it: bind the parameters, never splice them into the SQL;
 * apply every session property; run the SQL unchanged on the engine of the dialect asked for.
 */
public final class CompiledQuery {
  private final String sql;
  private final List<Object> parameters;
  private final List<Column> columns;
  private final Map<String, String> sessionProperties;
  private final String fingerprint;
  private final String kernelVersion;
  private final String modelVersion;
  private final String modelDigest;

  CompiledQuery(Compiled compiled, Identity identity) {
    this.sql = compiled.getSql();
    List<Object> values = new ArrayList<>(compiled.getParamsCount());
    for (Parameter parameter : compiled.getParamsList()) {
      switch (parameter.getValueCase()) {
        case TEXT:
          values.add(parameter.getText());
          break;
        case NUMBER:
          values.add(parameter.getNumber());
          break;
        case BOOLEAN:
          values.add(parameter.getBoolean());
          break;
        case NULL:
          values.add(null);
          break;
        default:
          // A kind this client does not know must not be bound as something else.
          throw new IllegalStateException("The service returned a parameter of an unknown kind");
      }
    }
    this.parameters = Collections.unmodifiableList(values);
    List<Column> described = new ArrayList<>(compiled.getColumnsCount());
    for (OutputColumn column : compiled.getColumnsList()) {
      described.add(new Column(column));
    }
    this.columns = Collections.unmodifiableList(described);
    Map<String, String> properties = new LinkedHashMap<>();
    for (SessionProperty property : compiled.getSessionPropertiesList()) {
      properties.put(property.getName(), property.getValue());
    }
    this.sessionProperties = Collections.unmodifiableMap(properties);
    StringBuilder hex = new StringBuilder();
    for (byte value : compiled.getFingerprint().toByteArray()) {
      hex.append(String.format("%02x", value));
    }
    this.fingerprint = hex.toString();
    this.kernelVersion = identity.getKernelVersion();
    this.modelVersion = identity.getModelVersion();
    this.modelDigest = identity.getModelDigest();
  }

  /** The statement, with {@code ?} where a parameter goes. */
  public String sql() {
    return sql;
  }

  /** One value per placeholder, in order: a String, a Double, a Boolean or null. */
  public List<Object> parameters() {
    return parameters;
  }

  /** The columns of the result, in order. */
  public List<Column> columns() {
    return columns;
  }

  /** Engine session settings the SQL needs; the executor applies every one. */
  public Map<String, String> sessionProperties() {
    return sessionProperties;
  }

  /**
   * Identifies this SQL for this model content, query, dialect, timezone, security filters and
   * kernel version, as hex. A key for a cache of results or of compilations. It changes with
   * every kernel version.
   */
  public String fingerprint() {
    return fingerprint;
  }

  public String kernelVersion() {
    return kernelVersion;
  }

  public String modelVersion() {
    return modelVersion;
  }

  /** The digest of the model content that was compiled against. */
  public String modelDigest() {
    return modelDigest;
  }

  /** Binds every parameter to {@code statement}, in order, by its type. */
  public void bind(PreparedStatement statement) throws SQLException {
    for (int index = 0; index < parameters.size(); index++) {
      Object value = parameters.get(index);
      if (value == null) {
        statement.setNull(index + 1, Types.NULL);
      } else if (value instanceof String) {
        statement.setString(index + 1, (String) value);
      } else if (value instanceof Double) {
        statement.setDouble(index + 1, (Double) value);
      } else {
        statement.setBoolean(index + 1, (Boolean) value);
      }
    }
  }

  /** A column of the result and the member of the model it stands for. */
  public static final class Column {
    private final String alias;
    private final String member;
    private final String type;
    private final String granularity;

    Column(OutputColumn column) {
      this.alias = column.getAlias();
      this.member = column.getMember();
      this.type = column.getType();
      this.granularity = column.hasGranularity() ? column.getGranularity() : null;
    }

    /** The column's name in the result set. */
    public String alias() {
      return alias;
    }

    /** The member of the model, such as {@code policies.written_premium}. */
    public String member() {
      return member;
    }

    /** {@code string}, {@code number}, {@code boolean} or {@code time}. */
    public String type() {
      return type;
    }

    /** The granularity of a time column, or null. */
    public String granularity() {
      return granularity;
    }
  }
}
