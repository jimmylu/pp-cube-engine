"""Generates the gRPC stubs from the service's contract in this checkout.

    python clients/python/generate.py

There is no copy of the contract here: the stubs are made from
crates/pp-cube-service/proto/engine.proto and written to
src/ppcube/_contract/, which is not kept in the repository. Building or
installing the package runs this first; it needs grpcio-tools.
"""
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CONTRACT = HERE.parents[1] / 'crates' / 'pp-cube-service' / 'proto'
TARGET = HERE / 'src' / 'ppcube' / '_contract'


def generate():
    from grpc_tools import protoc
    import grpc_tools

    if not (CONTRACT / 'engine.proto').is_file():
        if (TARGET / 'engine_pb2_grpc.py').is_file():
            # An installer that copies the package away before building it takes the
            # stubs along; they were generated from the contract where it was.
            return
        raise SystemExit(
            'generate: the contract is not at ' + str(CONTRACT) + '.\n'
            'Build the client from a checkout of the repository. If you did, your pip copied\n'
            'the package elsewhere before building it: use pip 21.3 or later, or run\n'
            '`python clients/python/generate.py` (needs grpcio-tools) and install again.')
    TARGET.mkdir(parents=True, exist_ok=True)
    include = Path(grpc_tools.__file__).parent / '_proto'
    status = protoc.main([
        'protoc', '--proto_path=' + str(CONTRACT), '--proto_path=' + str(include),
        '--python_out=' + str(TARGET), '--pyi_out=' + str(TARGET),
        '--grpc_python_out=' + str(TARGET), 'engine.proto'])
    if status != 0:
        raise SystemExit('generate: protoc failed')
    # protoc writes an absolute import of the sibling module; inside a package it is relative.
    stubs = TARGET / 'engine_pb2_grpc.py'
    text = stubs.read_text()
    absolute = 'import engine_pb2 as engine__pb2'
    if absolute not in text:
        raise SystemExit('generate: the generated stubs changed shape; update generate.py')
    stubs.write_text(text.replace(absolute, 'from . import engine_pb2 as engine__pb2'))
    (TARGET / '__init__.py').write_text(
        '"""Generated from crates/pp-cube-service/proto/engine.proto; not kept in the repository."""\n')


if __name__ == '__main__':
    generate()
    sys.stdout.write('generated ' + str(TARGET.relative_to(HERE)) + '\n')
