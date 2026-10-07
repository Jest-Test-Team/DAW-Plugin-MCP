"""MCP stdio client for Robot Framework. Speaks Content-Length JSON-RPC."""

from __future__ import annotations

import json
import os
import subprocess
import threading
from pathlib import Path


def _workspace() -> Path:
    return Path(__file__).resolve().parents[3]


class McpClient:
    ROBOT_LIBRARY_SCOPE = "SUITE"

    def __init__(self):
        self._proc: subprocess.Popen | None = None
        self._lock = threading.Lock()
        self._http_port: int | None = None

    def start_server(self, host: str = "reaper"):
        self.stop_server()
        root = _workspace()
        bin_path = root / "target" / "debug" / "daw-mcp"
        if not bin_path.exists():
            subprocess.check_call(["cargo", "build", "-p", "daw-mcp"], cwd=root)
        self._proc = subprocess.Popen(
            [str(bin_path), f"--host={host}"],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
        )
        init = self.call("initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "robot", "version": "0"}})
        if "error" in init:
            raise RuntimeError(init)
        return init["result"]["serverInfo"]["name"]

    def start_http_server(self, host: str = "reaper", port: int = 8765):
        self.stop_server()
        root = _workspace()
        bin_path = root / "target" / "debug" / "daw-mcp"
        if not bin_path.exists():
            subprocess.check_call(["cargo", "build", "-p", "daw-mcp"], cwd=root)
        self._http_port = int(port)
        self._proc = subprocess.Popen(
            [str(bin_path), f"--host={host}", f"--http={port}"],
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            bufsize=0,
        )
        import time
        import urllib.error
        import urllib.request

        url = f"http://127.0.0.1:{port}/health"
        last_err = None
        for _ in range(50):
            try:
                with urllib.request.urlopen(url, timeout=1) as resp:
                    body = resp.read().decode("utf-8")
                    if resp.status == 200 and "127.0.0.1" in body:
                        return body
            except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
                last_err = e
                time.sleep(0.1)
        err = last_err
        if self._proc and self._proc.poll() is not None:
            stderr = self._proc.stderr.read() if self._proc.stderr else b""
            err = f"{last_err}; process exited {self._proc.returncode}: {stderr!r}"
        raise RuntimeError(f"HTTP server did not start: {err}")

    def http_get(self, path: str = "/health"):
        import urllib.request

        port = self._http_port or 8765
        url = f"http://127.0.0.1:{port}{path}"
        with urllib.request.urlopen(url, timeout=5) as resp:
            return resp.status, resp.read().decode("utf-8")

    def response_leaks_home(self, text: str) -> bool:
        home = os.path.expanduser("~")
        return bool(home) and home in text

    def stop_server(self):
        self._http_port = None
        if self._proc:
            self._proc.kill()
            try:
                self._proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self._proc.kill()
            self._proc = None

    def call(self, method: str, params=None, rpc_id: int = 1):
        if not self._proc or not self._proc.stdin or not self._proc.stdout:
            raise RuntimeError("server not started")
        msg = {"jsonrpc": "2.0", "id": rpc_id, "method": method}
        if params is not None:
            msg["params"] = params
        body = json.dumps(msg).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
        with self._lock:
            self._proc.stdin.write(header + body)
            self._proc.stdin.flush()
            return self._read()

    def call_tool(self, name: str, arguments=None):
        if arguments is None:
            arguments = {}
        elif isinstance(arguments, str):
            arguments = json.loads(arguments) if arguments.strip() else {}
        resp = self.call("tools/call", {"name": name, "arguments": arguments}, rpc_id=2)
        if "error" in resp:
            raise AssertionError(resp)
        return resp["result"]

    def tool_is_error(self, result) -> bool:
        return bool(result.get("isError"))

    def structured(self, result):
        return result.get("structuredContent")

    def _read(self):
        stdout = self._proc.stdout
        headers = b""
        while b"\r\n\r\n" not in headers:
            chunk = stdout.read(1)
            if not chunk:
                stderr = self._proc.stderr.read() if self._proc.stderr else b""
                raise RuntimeError(f"EOF from daw-mcp: {stderr!r}")
            headers += chunk
        header_text, _ = headers.split(b"\r\n\r\n", 1)
        length = None
        for line in header_text.decode("ascii", errors="replace").split("\r\n"):
            if line.lower().startswith("content-length:"):
                length = int(line.split(":", 1)[1].strip())
        if length is None:
            raise RuntimeError(f"no Content-Length: {header_text!r}")
        body = stdout.read(length)
        return json.loads(body.decode("utf-8"))
