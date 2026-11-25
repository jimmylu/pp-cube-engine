"""The client and what it returns. The generated stubs are in `ppcube._contract`."""
from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple, Union

import grpc

from ._contract import engine_pb2, engine_pb2_grpc

Query = Union[str, Mapping[str, Any]]
_KINDS = {
    engine_pb2.FAILURE_KIND_MODEL: 'model',
    engine_pb2.FAILURE_KIND_QUERY: 'query',
    engine_pb2.FAILURE_KIND_UNSUPPORTED: 'unsupported',
    engine_pb2.FAILURE_KIND_INTERNAL: 'internal',
}


class CompileRefused(Exception):
    """The service answered, and the answer is that there is no SQL.

    This is not a failed call: the same request gives the same answer, so do
    not retry it. `kind` says whose it is to fix:

    - ``model``: the model version is invalid or outside what the kernel compiles;
    - ``query``: the query is wrong for this model or dialect;
    - ``unsupported``: the kernel or the dialect does not implement the feature;
    - ``internal``: the kernel failed; report it.

    The message names members and rules of the model, never SQL or tables, and
    may be shown to whoever wrote the query.
    """

    def __init__(self, kind: str, message: str):
        super().__init__(message)
        self.kind = kind
        self.message = message


@dataclass(frozen=True)
class Column:
    """A column of the result and the member of the model it stands for."""

    alias: str
    """The column's name in the result set."""
    member: str
    """The member of the model, such as ``policies.written_premium``."""
    type: str
    """``string``, ``number``, ``boolean`` or ``time``."""
    granularity: Optional[str]
    """The granularity of a time column, or None."""


@dataclass(frozen=True)
class CompiledQuery:
    """SQL the kernel compiled, with everything needed to run it as it is.

    Three rules for whoever executes it: pass the parameters to the driver,
    never write them into the SQL; apply every session property; run the SQL
    unchanged on the engine of the dialect asked for.
    """

    sql: str
    """One statement, with ``?`` where a parameter goes."""
    parameters: Tuple[Any, ...]
    """One value per placeholder, in order: a str, a float, a bool or None."""
    columns: Tuple[Column, ...]
    session_properties: Dict[str, str]
    """Engine session settings the SQL needs; the executor applies every one."""
    fingerprint: str
    """Hex. Identifies this SQL for this model content, query, dialect, timezone,
    security filters and kernel version: a cache key. It changes with every
    kernel version."""
    kernel_version: str
    model_version: str
    model_digest: str

    def sql_as(self, paramstyle: str) -> str:
        """The statement for a driver of another DB-API ``paramstyle``.

        ``qmark`` is the SQL as it is. ``format`` writes ``%s`` for each
        placeholder and doubles every other ``%``, for drivers such as PyMySQL;
        ``numeric`` writes ``:1``, ``:2``. Quoted text and quoted identifiers
        are left alone, so a ``?`` or ``%`` inside a literal stays what it was.
        Pass `parameters` to the driver as before.
        """
        if paramstyle == 'qmark':
            return self.sql
        if paramstyle not in ('format', 'numeric'):
            raise ValueError('paramstyle must be qmark, format or numeric')
        out: List[str] = []
        quote: Optional[str] = None
        placed = 0
        position = 0
        while position < len(self.sql):
            character = self.sql[position]
            if quote:
                if character == quote and self.sql[position + 1:position + 2] == quote:
                    # A doubled quote is the quote itself, inside the quoted text.
                    out.append(character * 2)
                    position += 2
                    continue
                if character == quote:
                    quote = None
            elif character in ('\'', '"', '`'):
                quote = character
            elif character == '?':
                placed += 1
                out.append('%s' if paramstyle == 'format' else ':%d' % placed)
                position += 1
                continue
            if character == '%' and paramstyle == 'format':
                # The driver formats the statement, inside quotes too.
                out.append('%%')
            else:
                out.append(character)
            position += 1
        if quote or placed != len(self.parameters):
            # The scan and the kernel disagree about the statement: bind nothing by guess.
            raise ValueError('The statement could not be rewritten for ' + paramstyle)
        return ''.join(out)


def _parameter(parameter) -> Any:
    kind = parameter.WhichOneof('value')
    if kind == 'text':
        return parameter.text
    if kind == 'number':
        return parameter.number
    if kind == 'boolean':
        return parameter.boolean
    if kind == 'null':
        return None
    # A kind this client does not know must not be bound as something else.
    raise RuntimeError('The service returned a parameter of an unknown kind')


class _Bearer(grpc.UnaryUnaryClientInterceptor):
    def __init__(self, token: str):
        self._header = ('authorization', 'Bearer ' + token)

    def intercept_unary_unary(self, continuation, details, request):
        metadata = list(details.metadata or ()) + [self._header]
        return continuation(details._replace(metadata=metadata), request)


class Client:
    """A client of the pp-cube-engine compile service.

    It holds one connection and may be shared between threads; make one and
    keep it. A failed call (an unknown model version, a refused token, a full
    or unreachable service) is a `grpc.RpcError` with its status code; a
    request the service answered without SQL is `CompileRefused`.

    :param target: the service, such as ``engine.internal:50051``.
    :param token: the read token, sent as ``authorization: Bearer``.
    :param trust: a PEM file of certificates to trust instead of the system's,
        for a service whose certificate comes from a private authority.
    :param plaintext: no encryption. The service allows it on loopback only,
        where it needs no token; a token is never sent over it.
    :param deadline: seconds one call may take. The service gives up a
        compilation after ten.
    :param channel: a channel the caller manages, instead of `target`; it is
        not closed.
    """

    def __init__(self, target: Optional[str] = None, *, token: Optional[str] = None,
                 trust: Union[str, Path, None] = None, plaintext: bool = False,
                 deadline: float = 15.0, channel: Optional[grpc.Channel] = None):
        token = token.strip() if token else None
        if plaintext and token:
            # A token over an unencrypted connection is a token given away.
            raise ValueError('A token is not sent over a plaintext connection')
        self._owned = channel is None
        if channel is None:
            if not target:
                raise ValueError('A target or a channel is needed')
            if plaintext:
                channel = grpc.insecure_channel(target)
            else:
                roots = Path(trust).read_bytes() if trust else None
                channel = grpc.secure_channel(target, grpc.ssl_channel_credentials(roots))
        self._channel = channel
        if token:
            channel = grpc.intercept_channel(channel, _Bearer(token))
        self._stub = engine_pb2_grpc.CubeEngineStub(channel)
        self._deadline = deadline

    def compile(self, model_version: str, dialect: str, query: Query, *,
                timezone: str = 'UTC', security_filters: Sequence[Query] = (),
                request_id: str = '') -> CompiledQuery:
        """Compiles a Cube query to SQL.

        :param query: the query in Cube's format, as a mapping or as JSON text.
        :param security_filters: row-level rules, one Cube filter each. The
            kernel applies every one on top of the query, where the query
            cannot widen it. Only the system that enforces access may choose
            them: never take them from an end user.
        :param request_id: an id of the caller's own, returned in the
            service's log; not part of the fingerprint.
        :raises CompileRefused: when the service answers without SQL.
        :raises grpc.RpcError: NOT_FOUND for an unknown model version,
            UNAUTHENTICATED for a missing or wrong token, RESOURCE_EXHAUSTED
            when the service is full, DEADLINE_EXCEEDED, UNAVAILABLE.
        """
        as_json = lambda value: value if isinstance(value, str) else json.dumps(value)
        response = self._stub.Compile(engine_pb2.CompileRequest(
            model_version=model_version, query_json=as_json(query), dialect=dialect,
            timezone=timezone, security_filters_json=[as_json(rule) for rule in security_filters],
            request_id=request_id), timeout=self._deadline)
        result = response.WhichOneof('result')
        if result == 'failure':
            # A kind added to the contract later is not the caller's to fix.
            raise CompileRefused(_KINDS.get(response.failure.kind, 'internal'), response.failure.message)
        if result != 'compiled':
            raise RuntimeError('The service returned neither SQL nor a failure')
        compiled, identity = response.compiled, response.identity
        return CompiledQuery(
            sql=compiled.sql,
            parameters=tuple(_parameter(parameter) for parameter in compiled.params),
            columns=tuple(Column(column.alias, column.member, column.type,
                                 column.granularity if column.HasField('granularity') else None)
                          for column in compiled.columns),
            session_properties={p.name: p.value for p in compiled.session_properties},
            fingerprint=compiled.fingerprint.hex(),
            kernel_version=identity.kernel_version,
            model_version=identity.model_version,
            model_digest=identity.model_digest)

    def meta(self, model_version: str):
        """The public cubes, views and members of a model version, as the
        generated ``GetMetaResponse``; never SQL or tables."""
        return self._stub.GetMeta(engine_pb2.GetMetaRequest(model_version=model_version),
                                  timeout=self._deadline)

    def close(self) -> None:
        """Closes the connection, unless the caller gave the channel."""
        if self._owned:
            self._channel.close()

    def __enter__(self) -> 'Client':
        return self

    def __exit__(self, *exception) -> None:
        self.close()
