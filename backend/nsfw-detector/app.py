import io

from fastapi import FastAPI, Request, Response
from PIL import Image
from transformers import pipeline

MAX_BODY_BYTES = 16 * 1024 * 1024

BLOCKED_LABELS = [
    "feces or excrement",
    "gore with blood and open wounds",
    "a mutilated or dismembered body",
    "a corpse or dead body",
    "vomit",
    "a nazi swastika or hate symbol",
    "graphic violence or cruelty",
]

NEUTRAL_LABELS = [
    "a portrait of a person",
    "an animal",
    "a landscape or building",
    "food on a plate",
    "a logo or abstract art",
    "a cartoon or illustration",
    "an everyday object",
    "a video game screenshot",
]

app = FastAPI()
nsfw_classifier = pipeline("image-classification", model="/model")
concept_classifier = pipeline("zero-shot-image-classification", model="/clip")


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

    nsfw_results = nsfw_classifier(image)
    nsfw = next((r["score"] for r in nsfw_results if r["label"] == "nsfw"), 0.0)

    concept_results = concept_classifier(
        image, candidate_labels=BLOCKED_LABELS + NEUTRAL_LABELS
    )
    blocked = sum(
        r["score"] for r in concept_results if r["label"] in BLOCKED_LABELS
    )
    top_blocked = max(
        (r for r in concept_results if r["label"] in BLOCKED_LABELS),
        key=lambda r: r["score"],
        default=None,
    )

    return {
        "nsfw": max(nsfw, blocked),
        "sexual": nsfw,
        "blocked": blocked,
        "topBlockedLabel": top_blocked["label"] if top_blocked else None,
    }
