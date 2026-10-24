import socket
listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(("8.8.8.8", 53))
listener.listen(128)
while True:
    connection, _ = listener.accept()
    connection.close()
