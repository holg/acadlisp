#!/usr/bin/env python3
"""
Simple HTTP server with COOP/COEP headers for SharedArrayBuffer support.
Run: python3 serve_coop.py
Then open: http://localhost:8080
"""

import http.server
import socketserver

PORT = 8053

class COOPCOEPHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, directory=None, **kwargs):
        super().__init__(*args, directory="dist", **kwargs)

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

with socketserver.TCPServer(("", PORT), COOPCOEPHandler) as httpd:
    print(f"Serving at http://localhost:{PORT} with COOP/COEP headers")
    print("SharedArrayBuffer will be available for XLISP")
    httpd.serve_forever()
