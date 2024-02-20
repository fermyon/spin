from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse
import sys


class MyRequestHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Write logs to stdout instead of stderr
        log_entry = "[%s] %s\n" % (self.log_date_time_string(), format % args)
        sys.stdout.write(log_entry)

    def do_GET(self):
        incoming_path = urlparse(self.path).path
        with open('responses.txt', 'r') as file:
            for line in file:
                path, body = line.strip().split(' ', 1)
                if path == incoming_path:
                    self.send_response(200)
                    self.send_header('Content-type', 'text/plain')
                    self.end_headers()
                    self.wfile.write(body.encode())
                    return

        # If the requested path is not found, return a 404 response
        self.send_response(404)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'Not Found')


def run():
    server_address = ('', 0)
    httpd = HTTPServer(server_address, MyRequestHandler)
    port = httpd.server_address[1]
    print(f'Starting http server...')
    print(f'PORT=(80,{port})')
    print(f'READY',  flush=True)
    httpd.serve_forever()


if __name__ == '__main__':
    run()
