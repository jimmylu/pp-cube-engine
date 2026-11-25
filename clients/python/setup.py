"""Builds the package after generating its stubs from the contract; see generate.py."""
import importlib.util
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py

HERE = Path(__file__).resolve().parent


def generate():
    spec = importlib.util.spec_from_file_location('ppcube_generate', HERE / 'generate.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.generate()


class BuildWithStubs(build_py):
    def run(self):
        generate()
        # The stubs were written after the package list was read.
        self.packages = sorted(set(self.packages) | {'ppcube._contract'})
        super().run()


setup(cmdclass={'build_py': BuildWithStubs})
