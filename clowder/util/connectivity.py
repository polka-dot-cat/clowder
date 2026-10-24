"""Network connectivity

.. codeauthor:: Joe DeCapo <joe@polka.cat>

"""

import socket
from functools import wraps
from pathlib import Path
from subprocess import CompletedProcess
from time import sleep
from typing import Optional

import clowder.util.command as cmd


class NetworkConnectionError(Exception):
    pass


def enable_network_connection(gateway_address: Optional[str]) -> [CompletedProcess]:
    path = Path()
    from sys import platform
    results = []
    if platform == "linux":
        result = cmd.run("ip link set eth0 up", path)
        results.append(result)
        if gateway_address is not None:
            result = cmd.run(f"route add default gw {gateway_address}", path)
            results.append(result)
    elif platform == "darwin":
        result = cmd.run("networksetup -setairportpower airport on", path, check=False)
        sleep(5)
    elif platform == "win32":
        raise NotImplementedError
    else:
        raise NotImplementedError
    return result


def disable_network_connection() -> CompletedProcess:
    path = Path()
    from sys import platform
    if platform == "linux":
        result = cmd.run("ip link set eth0 down", path)
    elif platform == "darwin":
        result = cmd.run("networksetup -setairportpower airport off", path)
        sleep(1)
    elif platform == "win32":
        raise NotImplementedError
    else:
        raise NotImplementedError
    return result


def get_gateway_ip_address() -> Optional[str]:
    path = Path()
    from sys import platform
    if platform == "linux":
        result = cmd.run("netstat -nr | awk '{print $2}' | head -n3 | tail -n1", path, print_output=False)
        return result.stdout.strip()
    elif platform == "darwin":
        return None
    elif platform == "win32":
        raise NotImplementedError
    else:
        raise NotImplementedError


def is_offline(host: str = '8.8.8.8', port: int = 53, timeout: int = 3) -> bool:
    """Returns True if offline, False otherwise

    Service: domain (DNS/TCP)

    .. note:: Implementation source https://stackoverflow.com/a/33117579

    :param str host: Host to check. Default is 8.8.8.8 (google-public-dns-a.google.com)
    :param int port: Port number. Default is 53/tcp
    :param int timeout: Seconds to wait until timeout
    :return: True, if offline
    """

    try:
        socket.setdefaulttimeout(timeout)
        socket.socket(socket.AF_INET, socket.SOCK_STREAM).connect((host, port))
        return False
    except socket.error:
        return True


def network_connection_required(func):
    """If no network connection, print offline message and exit

    :raise NetworkConnectionError:
    """

    @wraps(func)
    def wrapper(*args, **kwargs):
        """Wrapper"""

        if is_offline():
            raise NetworkConnectionError('No available internet connection')
        return func(*args, **kwargs)

    return wrapper
