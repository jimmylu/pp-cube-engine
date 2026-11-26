"""A kit for testing the compile service by hand against real engines.

It compiles Cube queries through the service, executes the SQL on Presto, Doris
or Druid as a caller would, compares rows with expected answers and writes a
report. See docs/integration-testing.zh-CN.md.
"""
__version__ = '0.2.0'
