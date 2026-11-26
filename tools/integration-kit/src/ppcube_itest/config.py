"""The environment under test, from one INI file. Secrets are never in it:
a password is named by the environment variable that holds it, a token by its file."""
import configparser
from dataclasses import dataclass, field
import os
from pathlib import Path
import re
from typing import Dict, Optional

DIALECTS = ('presto', 'doris', 'druid')
NAME = re.compile(r'[a-z][a-z0-9_]*$')


class ConfigError(Exception):
    pass


@dataclass(frozen=True)
class Service:
    target: str
    ca: Optional[str]
    token_file: Optional[str]
    plaintext: bool
    deadline: float

    def token(self) -> Optional[str]:
        return Path(self.token_file).read_text(encoding='utf-8') if self.token_file else None


@dataclass(frozen=True)
class Engine:
    name: str
    dialect: str
    model_version: str
    data_source: str
    schema: str
    table_prefix: str
    options: Dict[str, str] = field(default_factory=dict)

    def option(self, key: str, default: Optional[str] = None) -> Optional[str]:
        return self.options.get(key, default)

    def password(self) -> Optional[str]:
        variable = self.option('password_env')
        if not variable:
            return None
        if variable not in os.environ:
            raise ConfigError('[engine:%s] password_env names %s, which is not set' % (self.name, variable))
        return os.environ[variable]

    def table(self, name: str) -> str:
        """A test table's name on the engine, without its schema."""
        return self.table_prefix + name


@dataclass(frozen=True)
class Config:
    service: Service
    engines: Dict[str, Engine]

    def engine(self, name: Optional[str]) -> Engine:
        if name is None and len(self.engines) == 1:
            return next(iter(self.engines.values()))
        if name not in self.engines:
            raise ConfigError('--engine must be one of: ' + ', '.join(sorted(self.engines)))
        return self.engines[name]


def load(path: str) -> Config:
    parser = configparser.ConfigParser(interpolation=None)
    if not parser.read(path, encoding='utf-8'):
        raise ConfigError('The configuration file %s cannot be read' % path)
    if 'service' not in parser:
        raise ConfigError('The configuration needs a [service] section')
    section = parser['service']
    if not section.get('target'):
        raise ConfigError('[service] needs target, such as host:50051')
    service = Service(
        target=section['target'], ca=section.get('ca') or None,
        token_file=section.get('token_file') or None,
        plaintext=section.getboolean('plaintext', fallback=False),
        deadline=section.getfloat('deadline', fallback=20.0))
    if service.plaintext and service.token_file:
        raise ConfigError('[service] plaintext and token_file do not go together: a token is never sent unencrypted')
    engines = {}
    for title in parser.sections():
        if not title.startswith('engine:'):
            continue
        name, section = title[len('engine:'):], parser[title]
        problem = lambda text: ConfigError('[%s] %s' % (title, text))
        dialect = section.get('dialect', '')
        if dialect not in DIALECTS:
            raise problem('dialect must be presto, doris or druid')
        if not section.get('model_version'):
            raise problem('needs model_version, such as itest/presto/v1')
        # Druid has one schema; its test datasources are told apart by a prefix.
        schema = 'druid' if dialect == 'druid' else section.get('schema', '')
        prefix = section.get('table_prefix', 'ppcube_itest_' if dialect == 'druid' else '')
        if not NAME.match(schema) or (prefix and not NAME.match(prefix)):
            raise problem('schema and table_prefix are lower-case letters, digits and _')
        if dialect == 'druid' and not prefix:
            raise problem('table_prefix must not be empty on Druid: its datasources share one schema')
        known = {'dialect', 'model_version', 'data_source', 'schema', 'table_prefix'}
        engines[name] = Engine(
            name=name, dialect=dialect, model_version=section['model_version'],
            data_source=section.get('data_source', 'warehouse'), schema=schema, table_prefix=prefix,
            options={key: value for key, value in section.items() if key not in known})
    if not engines:
        raise ConfigError('The configuration needs at least one [engine:<name>] section')
    return Config(service, engines)
