import io

from fastapi import FastAPI, Request, Response
from PIL import Image
from transformers import pipeline

MAX_BODY_BYTES = 16 * 1024 * 1024

app = FastAPI()
classifier = pipeline("image-classification", model="/model")


@app.get("/healthz")
async def healthz():
    return {"ok": True}


@app.post("/check")
async def check(request: Request):
    body = await request.body()
    if not body or len(body) > MAX_BODY_BYTES:
        return Response(status_code=400)
    try:
        image = Image.open(io.BytesIO(body)).convert("RGB")
    except Exception:
        return Response(status_code=400)
    results = classifier(image)
    nsfw = next((r["score"] for r in results if r["label"] == "nsfw"), 0.0)
    return {"nsfw": nsfw}
