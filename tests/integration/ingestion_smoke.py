"""Run a built node against deterministic RPC data and an isolated PostgreSQL schema."""
import http.server
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
DATABASE = os.environ["DATABASE_URL"]
SCHEMA = "sq_smoke_" + uuid.uuid4().hex
STATE = {"chain": "0x1", "fork": False}


def block_hash(height):
    return "0x" + format(height + 1, "064x")


class Rpc(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method, params = request["method"], request["params"]
        if method == "eth_chainId":
            result = STATE["chain"]
        elif method == "eth_blockNumber":
            result = "0x8"
        elif method == "eth_getBlockByNumber":
            height = 8 if params[0] == "finalized" else int(params[0], 16)
            result = {
                "number": hex(height), "hash": block_hash(height + (100 if STATE["fork"] else 0)),
                "parentHash": block_hash(height - 1), "timestamp": hex(1700000000 + height),
                "transactions": [],
            }
        elif method == "eth_getLogs":
            assert "blockHash" in params[0]
            result = []
        else:
            raise AssertionError(method)
        body = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def sql(statement):
    return subprocess.check_output(["psql", DATABASE, "-XAt", "-v", "ON_ERROR_STOP=1", "-c", statement], text=True).strip()


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Rpc)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    with tempfile.TemporaryDirectory(prefix="sq-ingestion-") as tmp:
        project = Path(tmp) / "project"
        shutil.copytree(ROOT / "tests/fixtures/sdk-erc20", project)
        manifest = project / "project.yaml"
        original = manifest.read_text().replace("startBlock: 21000000", "startBlock: 0")
        manifest.write_text(original)
        url = DATABASE + ("&" if "?" in DATABASE else "?") + "schema=" + SCHEMA
        command = [str(ROOT / "target/debug/superquery-node"), "--ingest-only", "--project", str(project),
                   "--database-url", url, "--rpc-url", f"http://127.0.0.1:{server.server_port}",
                   "--rpc-max-retries", "2", "--rpc-timeout-secs", "2", "--no-admin"]

        def run(end, expected=0):
            result = subprocess.run(command + ["--end-height", str(end)], capture_output=True, text=True, timeout=30)
            if (result.returncode == 0) != (expected == 0):
                raise AssertionError(result.stdout + result.stderr)
            return result.stdout + result.stderr

        run(3)
        assert sql(f'SELECT count(*) FROM "{SCHEMA}"._superquery_ingestion_blocks') == "4"
        run(6)
        assert sql(f'SELECT count(*) FROM "{SCHEMA}"._superquery_ingestion_blocks') == "7"
        assert sql(f'SELECT count(*) FROM "{SCHEMA}"._superquery_metadata WHERE key = \'indexed_height\'') == "0"
        manifest.write_text(original.replace('version: "0.1.0"', 'version: "0.2.0"'))
        assert "manifest_hash" in run(8, expected=1)
        manifest.write_text(original)
        STATE["fork"] = True
        assert "no longer canonical" in run(8, expected=1)
        STATE["fork"] = False
        STATE["chain"] = "0x89"
        assert "chain id mismatch" in run(8, expected=1)
        STATE["chain"] = "0x1"
        with tempfile.TemporaryFile(mode="w+") as output:
            process = subprocess.Popen(command, stdout=output, stderr=output)
            try:
                deadline = time.monotonic() + 20
                while sql(f'SELECT count(*) FROM "{SCHEMA}"._superquery_ingestion_blocks') != "9":
                    if process.poll() is not None or time.monotonic() > deadline:
                        output.seek(0)
                        raise AssertionError(output.read())
                    time.sleep(0.1)
                process.send_signal(signal.SIGTERM)
                assert process.wait(timeout=5) == 0
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
        run(8)
        assert sql(f'SELECT count(*) FROM "{SCHEMA}"._superquery_ingestion_blocks') == "9"
        print("PASS: ingestion, restart, independent cursor, changed project, wrong chain, fork refusal, SIGTERM")
finally:
    sql(f'DROP SCHEMA IF EXISTS "{SCHEMA}" CASCADE')
    server.shutdown()
    server.server_close()
