import http.server
import ssl
import os

os.chdir(os.path.dirname(os.path.abspath(__file__)))

server_address = ('0.0.0.0', 8443)
httpd = http.server.HTTPServer(server_address, http.server.SimpleHTTPRequestHandler)

ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain('certs/cert.pem', 'certs/key.pem')
httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)

print(f"Serving on https://0.0.0.0:8443")
httpd.serve_forever()
