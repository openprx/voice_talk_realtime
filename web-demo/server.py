import http.server
import ssl
import os

os.chdir(os.path.dirname(os.path.abspath(__file__)))

class NoCacheHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store, no-cache, must-revalidate, max-age=0')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        super().end_headers()

server_address = ('0.0.0.0', 8443)
httpd = http.server.HTTPServer(server_address, NoCacheHandler)

ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain('certs/cert.pem', 'certs/key.pem')
httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)

print('Serving on https://0.0.0.0:8443')
httpd.serve_forever()
