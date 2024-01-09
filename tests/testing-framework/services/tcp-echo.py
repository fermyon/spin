import socket
import threading
import os


def handle_client(client_socket):
    while True:
        data = client_socket.recv(1024)
        if not data:
            break
        # Echo the received data back to the client
        client_socket.send(data)
    client_socket.close()


def echo_server():
    host = "127.0.0.1"
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_socket.bind((host, 0))
    server_socket.listen(5)
    _, port = server_socket.getsockname()
    print(f"Listening on {host}...")
    print(f"PORT=(5000,{port})")
    print(f"READY", flush=True)

    try:
        while True:
            client_socket, client_address = server_socket.accept()
            print(f"Accepted connection from {client_address}")
            # Handle the client in a separate thread
            client_handler = threading.Thread(
                target=handle_client, args=(client_socket,))
            client_handler.start()
    except KeyboardInterrupt:
        print("Server shutting down.")
    finally:
        # Close the server socket
        server_socket.close()


if __name__ == "__main__":
    echo_server()
