package dev.ppcube.example;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.facebook.presto.jdbc.PrestoConnection;
import dev.ppcube.client.CubeEngineClient;
import dev.ppcube.client.Compilation;
import dev.ppcube.client.CompileRefusedException;
import dev.ppcube.client.CompiledQuery;
import dev.ppcube.engine.v1.CubeMeta;
import dev.ppcube.engine.v1.GetMetaResponse;
import io.grpc.Status;
import io.grpc.StatusRuntimeException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.time.Duration;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import java.util.stream.Collectors;

/**
 * The back end of an insurer's analytics portal, as far as the kernel is concerned.
 *
 * <p>For every user and every question on their dashboard it does what a real back end does:
 * decide the user's row-level rules, ask the compile service for SQL, run that SQL on the
 * warehouse with its parameters bound and its session properties applied, and show the rows.
 * The kernel never sees the warehouse, and the warehouse never sees a Cube query.
 *
 * <p>Configuration comes from the environment, as in a container; see {@code run.sh}.
 */
public final class Portal {
  private static final ObjectMapper JSON = new ObjectMapper();

  private final CubeEngineClient kernel;
  private final String warehouseUrl;
  private final JsonNode portal;
  /** Results by fingerprint: the same SQL for the same rules is run once. */
  private final Map<String, List<List<Object>>> results = new HashMap<>();
  private int asked;
  private int answered;
  private int refused;
  private int served;
  private final List<String> wrong = new ArrayList<>();

  private Portal(CubeEngineClient kernel, String warehouseUrl, JsonNode portal) {
    this.kernel = kernel;
    this.warehouseUrl = warehouseUrl;
    this.portal = portal;
  }

  public static void main(String[] arguments) throws Exception {
    String token =
        new String(Files.readAllBytes(Paths.get(env("PPCUBE_TOKEN_FILE"))), StandardCharsets.UTF_8);
    JsonNode portal = JSON.readTree(Paths.get(env("PORTAL_FILE")).toFile());
    String warehouseUrl = env("WAREHOUSE_URL");
    try (CubeEngineClient kernel =
        CubeEngineClient.to(env("PPCUBE_TARGET"))
            .trust(Paths.get(env("PPCUBE_CA")))
            .token(token)
            .deadline(Duration.ofSeconds(20))
            .build()) {
      Portal back = new Portal(kernel, warehouseUrl, portal);
      back.awaitWarehouse();
      if (System.getenv("WAREHOUSE_SQL") != null) {
        back.loadWarehouse(Paths.get(System.getenv("WAREHOUSE_SQL")));
      }
      back.describeModel();
      for (JsonNode user : portal.get("users")) {
        back.dashboard(user);
      }
      // Another session of a user already seen: the SQL is compiled again, cheaply,
      // and the fingerprint finds the rows without the warehouse.
      back.dashboard(portal.get("users").get(0));
      System.exit(back.summary());
    }
  }

  private static String env(String name) {
    String value = System.getenv(name);
    if (value == null || value.isEmpty()) {
      throw new IllegalStateException("Set " + name + "; run.sh sets it");
    }
    return value;
  }

  private Connection warehouse(Map<String, String> sessionProperties) throws SQLException {
    Properties properties = new Properties();
    properties.setProperty("user", "analytics-portal");
    Connection connection = DriverManager.getConnection(warehouseUrl, properties);
    // What the SQL needs of the engine, as the kernel declared it.
    PrestoConnection presto = connection.unwrap(PrestoConnection.class);
    for (Map.Entry<String, String> property : sessionProperties.entrySet()) {
      presto.setSessionProperty(property.getKey(), property.getValue());
    }
    return connection;
  }

  /** A warehouse that was just started answers a moment later than it accepts connections. */
  private void awaitWarehouse() throws Exception {
    SQLException last = null;
    for (int attempt = 0; attempt < 90; attempt++) {
      try (Connection connection = warehouse(new HashMap<>());
          Statement statement = connection.createStatement()) {
        statement.execute("CREATE SCHEMA IF NOT EXISTS memory.portal_ready");
        statement.execute("CREATE TABLE IF NOT EXISTS memory.portal_ready.probe AS SELECT 1 AS one");
        return;
      } catch (SQLException notYet) {
        last = notYet;
        Thread.sleep(2000);
      }
    }
    throw new IllegalStateException("The warehouse did not become ready", last);
  }

  private void loadWarehouse(Path file) throws Exception {
    int statements = 0;
    try (Connection connection = warehouse(new HashMap<>());
        Statement statement = connection.createStatement()) {
      for (String line : Files.readAllLines(file, StandardCharsets.UTF_8)) {
        String sql = line.trim();
        if (sql.isEmpty() || sql.startsWith("--")) {
          continue;
        }
        statement.execute(sql.substring(0, sql.length() - 1));
        statements++;
      }
    }
    System.out.printf("warehouse: loaded, %d statements%n%n", statements);
  }

  /** What the portal offers its users to choose from: names and titles, never SQL. */
  private void describeModel() throws Exception {
    String version = portal.get("modelVersion").asText();
    GetMetaResponse meta = null;
    // The service reads its store when it starts; a portal that starts beside it waits.
    for (int attempt = 0; meta == null; attempt++) {
      try {
        meta = kernel.meta(version);
      } catch (StatusRuntimeException notYet) {
        Status.Code code = notYet.getStatus().getCode();
        if (attempt == 60 || (code != Status.Code.UNAVAILABLE && code != Status.Code.NOT_FOUND)) {
          throw notYet;
        }
        Thread.sleep(1000);
      }
    }
    System.out.printf(
        "model %s (%s), kernel %s%n",
        meta.getIdentity().getModelVersion(),
        meta.getIdentity().getModelDigest().substring(0, 19),
        meta.getIdentity().getKernelVersion());
    for (CubeMeta cube : meta.getCubesList()) {
      System.out.printf(
          "  %-16s %d measures, %d dimensions%n",
          cube.getName(), cube.getMeasuresCount(), cube.getDimensionsCount());
    }
    System.out.println();
  }

  private void dashboard(JsonNode user) throws Exception {
    List<String> rules = new ArrayList<>();
    for (JsonNode rule : user.get("rowFilters")) {
      rules.add(JSON.writeValueAsString(rule));
    }
    System.out.printf("== %s (%s)%n", user.get("title").asText(), user.get("name").asText());
    System.out.printf("   row-level rules: %s%n%n", rules.isEmpty() ? "none" : String.join(" and ", rules));
    for (JsonNode question : user.get("questions")) {
      ask(user.get("name").asText(), rules, question);
    }
  }

  private void ask(String user, List<String> rules, JsonNode question) throws Exception {
    asked++;
    String name = question.get("name").asText();
    System.out.printf("-- %s%n", question.get("title").asText());
    Compilation.Builder compilation =
        Compilation.of(
                portal.get("modelVersion").asText(),
                portal.get("dialect").asText(),
                JSON.writeValueAsString(question.get("query")))
            .timezone(portal.get("timezone").asText())
            .requestId(user + "/" + name);
    for (String rule : rules) {
      compilation.securityFilter(rule);
    }
    CompiledQuery compiled;
    try {
      compiled = kernel.compile(compilation.build());
    } catch (CompileRefusedException noSql) {
      // The answer is final: the portal tells the user why, it does not retry.
      System.out.printf("   no SQL (%s): %s%n", noSql.kind(), noSql.getMessage());
      JsonNode known = question.get("refused");
      if (known != null && known.get("kind").asText().equals(noSql.kind().name())) {
        System.out.printf("   %s%n%n", known.get("then").asText());
        refused++;
      } else {
        wrong.add(user + "/" + name + ": refused, " + noSql.kind());
        System.out.println();
      }
      return;
    }
    if (question.has("refused")) {
      wrong.add(user + "/" + name + ": expected a refusal, got SQL");
    }
    List<List<Object>> rows = results.get(compiled.fingerprint());
    boolean cached = rows != null;
    if (cached) {
      served++;
    } else {
      rows = run(compiled);
      results.put(compiled.fingerprint(), rows);
    }
    System.out.printf(
        "   %d parameters bound, %d session properties, fingerprint %s%s%n",
        compiled.parameters().size(),
        compiled.sessionProperties().size(),
        compiled.fingerprint().substring(0, 12),
        cached ? ", rows from the cache" : "");
    print(compiled, rows);
    if (question.has("expect")) {
      String difference = difference(question.get("expect"), compiled, rows);
      if (difference == null) {
        answered++;
      } else {
        wrong.add(user + "/" + name + ": " + difference);
        System.out.printf("   WRONG: %s%n", difference);
      }
    } else {
      answered++;
    }
    System.out.println();
  }

  /** Runs the SQL as it is: parameters bound, never written into the text. */
  private List<List<Object>> run(CompiledQuery compiled) throws SQLException {
    List<List<Object>> rows = new ArrayList<>();
    try (Connection connection = warehouse(compiled.sessionProperties());
        PreparedStatement statement = connection.prepareStatement(compiled.sql())) {
      compiled.bind(statement);
      try (ResultSet result = statement.executeQuery()) {
        while (result.next()) {
          List<Object> row = new ArrayList<>();
          for (CompiledQuery.Column column : compiled.columns()) {
            // The kernel says what each column is; the driver need not be asked.
            Object value;
            if (column.type().equals("time")) {
              String text = result.getString(column.alias());
              value = text == null ? null : text.replace(' ', 'T').replaceFirst("\\.0+$", "");
            } else if (column.type().equals("number")) {
              double number = result.getDouble(column.alias());
              value = result.wasNull() ? null : (Object) number;
            } else {
              value = result.getObject(column.alias());
            }
            row.add(value);
          }
          rows.add(row);
        }
      }
    }
    return rows;
  }

  private static void print(CompiledQuery compiled, List<List<Object>> rows) {
    List<String> header =
        compiled.columns().stream().map(CompiledQuery.Column::member).collect(Collectors.toList());
    List<List<String>> lines = new ArrayList<>();
    lines.add(header);
    for (List<Object> row : rows) {
      lines.add(row.stream().map(Portal::shown).collect(Collectors.toList()));
    }
    int[] widths = new int[header.size()];
    for (List<String> line : lines) {
      for (int column = 0; column < widths.length; column++) {
        widths[column] = Math.max(widths[column], line.get(column).length());
      }
    }
    for (List<String> line : lines) {
      StringBuilder text = new StringBuilder("   ");
      for (int column = 0; column < widths.length; column++) {
        text.append(String.format("%-" + (widths[column] + 2) + "s", line.get(column)));
      }
      System.out.println(text.toString().replaceFirst("\\s+$", ""));
    }
  }

  private static String shown(Object value) {
    if (value == null) {
      return "-";
    }
    if (value instanceof Double) {
      double number = (Double) value;
      return number == Math.rint(number) ? String.format("%,d", (long) number) : String.format("%,.4f", number);
    }
    return value.toString();
  }

  /** Null when the rows are the expected ones; otherwise what differs. */
  private static String difference(JsonNode expect, CompiledQuery compiled, List<List<Object>> rows) {
    List<String> aliases =
        compiled.columns().stream().map(CompiledQuery.Column::alias).collect(Collectors.toList());
    List<String> columns = new ArrayList<>();
    expect.get("columns").forEach(column -> columns.add(column.asText()));
    if (!aliases.equals(columns)) {
      return "columns " + aliases + ", expected " + columns;
    }
    JsonNode expected = expect.get("rows");
    if (expected.size() != rows.size()) {
      return rows.size() + " rows, expected " + expected.size();
    }
    for (int row = 0; row < rows.size(); row++) {
      for (int column = 0; column < columns.size(); column++) {
        JsonNode want = expected.get(row).get(column);
        Object got = rows.get(row).get(column);
        boolean same;
        if (want.isNull()) {
          same = got == null;
        } else if (want.isNumber()) {
          same = got instanceof Double && Math.abs((Double) got - want.asDouble()) < 1e-9;
        } else if (want.isObject()) {
          same = want.get("timestamp").asText().equals(got);
        } else if (want.isBoolean()) {
          same = Boolean.valueOf(want.asBoolean()).equals(got);
        } else {
          same = want.asText().equals(got);
        }
        if (!same) {
          return "row " + (row + 1) + ", " + columns.get(column) + ": " + got + ", expected " + want;
        }
      }
    }
    return null;
  }

  private int summary() {
    System.out.printf(
        "%d questions asked: %d answered as expected, %d refused as expected, %d served from the cache%n",
        asked, answered, refused, served);
    if (wrong.isEmpty()) {
      System.out.println("example: PASSED");
      return 0;
    }
    wrong.forEach(line -> System.out.println("  " + line));
    System.out.println("example: FAILED");
    return 1;
  }
}
