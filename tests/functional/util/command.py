"""New syntax test file"""

import os
import re
import shlex
import sys
from pathlib import Path
from subprocess import CompletedProcess
from typing import List

import clowder.util.command as cmd
from clowder.util.console import CONSOLE


class CommandResults:
    def __init__(self):
        self.completed_processes: List[CompletedProcess] = []


def run_command(command: str, cwd: Path, check: bool = False) -> CompletedProcess:
    env = {"DEBUG": "true"}
    processed_command = _process_clowder_commands(command)
    CONSOLE.stdout('echo $PATH')
    result = cmd.run('echo $PATH', cwd=cwd, check=check, print_output=False, print_command=True, env=env)
    CONSOLE.stdout(result.stdout.strip())
    result = cmd.run(processed_command, cwd=cwd, check=check, print_output=False, print_command=True, env=env)

    output = result.stdout.strip()
    if output:
        print(output)
    CONSOLE.stdout(f'Return code: {result.returncode}')
    if check:
        assert result.returncode == 0
    return result


def _process_clowder_commands(command: str) -> str:

    pattern = r'^clowder(?=\s|$)'
    executable = os.environ.get('CLOWDER_EXECUTABLE')
    if executable:
        replacement = shlex.quote(executable)
    else:
        replacement = f'{shlex.quote(sys.executable)} -m clowder.app'
    output = re.sub(pattern, lambda match: replacement, command)

    return output
