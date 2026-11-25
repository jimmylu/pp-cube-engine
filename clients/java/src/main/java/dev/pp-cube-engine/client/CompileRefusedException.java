package dev.cindervale.client;

/**
 * The service answered, and the answer is that there is no SQL.
 *
 * <p>This is not a transport failure: retrying the same request gives the same answer. The
 * message names members and rules of the model, never SQL or tables, and may be shown to whoever
 * wrote the query.
 */
public final class CompileRefusedException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  /** Why there is no SQL. */
  public enum Kind {
    /** The model version is invalid or outside what the kernel compiles. Fix the model. */
    MODEL,
    /** The query is wrong for this model or dialect. Fix the query. */
    QUERY,
    /** A feature the kernel or the dialect does not implement. Do not retry. */
    UNSUPPORTED,
    /** The kernel failed. Report it; the message carries no detail by design. */
    INTERNAL
  }

  private final Kind kind;

  CompileRefusedException(Kind kind, String message) {
    super(message);
    this.kind = kind;
  }

  public Kind kind() {
    return kind;
  }
}
