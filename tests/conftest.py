"""Default to Swift; retain explicit Rust and Python comparison modes."""

import json
import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest


def pytest_configure(config):
    implementation = os.environ.get('CLOWDER_IMPLEMENTATION', 'swift')
    if implementation not in ('rust', 'swift', 'python'):
        raise pytest.UsageError('CLOWDER_IMPLEMENTATION must be rust, swift, or python')
    label = 'Swift' if implementation == 'swift' else 'Rust'
    executable = os.environ.get('CLOWDER_EXECUTABLE')
    if implementation == 'python':
        if executable:
            raise pytest.UsageError('Unset CLOWDER_EXECUTABLE when selecting the Python reference')
        # Aliases and shell scripts must use the reference implementation too,
        # even when a native clowder is already installed earlier on PATH.
        config.clowder_reference_directory = tempfile.TemporaryDirectory(prefix='clowder-python-reference-')
        directory = Path(config.clowder_reference_directory.name)
        launcher = directory / 'clowder'
        launcher.write_text(f'#!/bin/sh\nexec {shlex.quote(sys.executable)} -m clowder.app "$@"\n')
        launcher.chmod(0o700)
        os.environ['PATH'] = str(directory) + os.pathsep + os.environ.get('PATH', '')
        config.clowder_cli_description = f'Python reference ({sys.executable})'
        return
    if executable:
        path = Path(executable).expanduser().resolve()
        if not path.is_file() or not os.access(path, os.X_OK):
            raise pytest.UsageError(f'CLOWDER_EXECUTABLE is not an executable file: {path}')
        os.environ['CLOWDER_EXECUTABLE'] = str(path)
        config.clowder_cli_description = f'{label} ({path})'
        return
    if config.option.collectonly:
        config.clowder_cli_description = f'{label} (collection only; build deferred)'
        return
    # This runs before xdist starts its workers. They inherit the selected binary
    # through the environment, so workers do not start additional builds.
    root = Path(__file__).resolve().parent.parent
    if implementation == 'swift':
        try:
            subprocess.run(['swift', 'build', '--package-path', str(root / 'swift')], cwd=root, check=True)
            directory = subprocess.check_output(
                ['swift', 'build', '--package-path', str(root / 'swift'), '--show-bin-path'], cwd=root, text=True,
            ).strip()
        except (FileNotFoundError, subprocess.CalledProcessError) as error:
            raise pytest.UsageError('Swift tests require a working Swift 6.2 or later toolchain') from error
        executable = str(Path(directory) / 'clowder')
        os.environ['CLOWDER_EXECUTABLE'] = executable
        config.clowder_cli_description = f'Swift ({executable})'
        return
    try:
        result = subprocess.run(
            ['cargo', 'build', '--locked', '--message-format=json'],
            cwd=root, stdout=subprocess.PIPE, text=True, check=False,
        )
    except FileNotFoundError as error:
        raise pytest.UsageError('Rust tests require Cargo; install the pinned Rust toolchain') from error
    for line in result.stdout.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if event.get('reason') == 'compiler-message':
            rendered = event.get('message', {}).get('rendered')
            if rendered:
                print(rendered, file=sys.stderr, end='')
        if event.get('reason') == 'compiler-artifact' and event.get('target', {}).get('name') == 'clowder':
            executable = event.get('executable') or executable
    if result.returncode or not executable:
        raise pytest.UsageError('Cargo could not build the native Clowder executable')
    os.environ['CLOWDER_EXECUTABLE'] = str(Path(executable).resolve())
    config.clowder_cli_description = f'Rust ({executable})'


def pytest_report_header(config):
    return f'Clowder CLI: {config.clowder_cli_description}'


def pytest_unconfigure(config):
    directory = getattr(config, 'clowder_reference_directory', None)
    if directory is not None:
        directory.cleanup()
