"""A client of the pp-cube-engine compile service: a Cube query in, SQL and its parameters out.

    from ppcube import Client, CompileRefused

    with Client('engine.internal:50051', token=read_token) as kernel:
        compiled = kernel.compile('property/2026-11', 'presto', query,
                                  security_filters=[region_rule])
        cursor.execute(compiled.sql, compiled.parameters)

It executes nothing. See the README beside this package.
"""
from .client import Client, Column, CompiledQuery, CompileRefused

__all__ = ['Client', 'Column', 'CompiledQuery', 'CompileRefused']
__version__ = '0.2.0'
