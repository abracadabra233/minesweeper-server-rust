import argparse
import os
from pathlib import Path
from typing import List, Optional

import uvicorn
from fastapi import APIRouter, Body, Depends, FastAPI, File, Query, UploadFile, WebSocket
from fastapi.background import BackgroundTasks
from fastapi.responses import HTMLResponse, RedirectResponse

app = FastAPI(title="ASR api", openapi_url=f"/openapi.json")


@app.websocket("/server")
async def server(
    websocket: WebSocket,
    client_id: Optional[str] = Query(None),
):
    mpm_infer_service = MPMInferService()
    await mpm_infer_service.run(websocket, client_id)


if __name__ == "__main__":
    from pathlib import Path

    warkspace = Path(__file__).parent.parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--certfile", type=str, default=f"{warkspace}/ssl_key/server.crt", required=False)
    parser.add_argument("--keyfile", type=str, default=f"{warkspace}/ssl_key/server.key", required=False)
    args = parser.parse_args()
    uvicorn.run(app, host="0.0.0.0", port=15439, ssl_certfile=args.certfile, ssl_keyfile=args.keyfile)
