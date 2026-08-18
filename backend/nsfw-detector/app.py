import io
import os
import re
import threading

from fastapi import FastAPI, Request, Response
from fastapi.concurrency import run_in_threadpool
from PIL import Image
from transformers import pipeline

MAX_BODY_BYTES = 16 * 1024 * 1024
TEXT_MAX_LEN = 100
TEXT_TOXICITY_THRESHOLD = float(os.environ.get("TEXT_TOXICITY_THRESHOLD", "0.8"))

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

LEET_MAP = str.maketrans(
    {
        "4": "a",
        "@": "a",
        "1": "i",
        "!": "i",
        "0": "o",
        "3": "e",
        "5": "s",
        "$": "s",
        "6": "b",
        "7": "t",
        "8": "b",
        "9": "g",
        "2": "z",
    }
)

TRANSLIT_DIGRAPHS = [
    ("shch", "щ"),
    ("sch", "щ"),
    ("zh", "ж"),
    ("kh", "х"),
    ("ch", "ч"),
    ("sh", "ш"),
    ("yo", "ё"),
    ("yu", "ю"),
    ("ya", "я"),
    ("ja", "я"),
    ("ts", "ц"),
    ("ck", "к"),
    ("ph", "ф"),
]

TRANSLIT_SINGLE = str.maketrans(
    {
        "a": "а",
        "b": "б",
        "c": "с",
        "d": "д",
        "e": "е",
        "f": "ф",
        "g": "г",
        "h": "х",
        "i": "и",
        "j": "й",
        "k": "к",
        "l": "л",
        "m": "м",
        "n": "н",
        "o": "о",
        "p": "п",
        "q": "к",
        "r": "р",
        "s": "с",
        "t": "т",
        "u": "у",
        "v": "в",
        "w": "в",
        "x": "х",
        "y": "у",
        "z": "з",
    }
)

EN_BLOCKED = [
    "fuck",
    "shit",
    "cunt",
    "nigg",
    "faggot",
    "whore",
    "bitch",
    "penis",
    "pussy",
    "porn",
    "hitler",
    "nazi",
    "kike",
    "tranny",
    "slut",
    "rape",
    "cumshot",
    "blowjob",
    "handjob",
    "dildo",
    "asshole",
    "retard",
    "twat",
    "wank",
    "jizz",
]

RU_BLOCKED = [
    "хуй",
    "хуи",
    "хуя",
    "хуе",
    "пизд",
    "ебан",
    "ебат",
    "ебал",
    "ебл",
    "заеб",
    "уебок",
    "уебан",
    "бляд",
    "блят",
    "сука",
    "суки",
    "сучк",
    "мудак",
    "мудил",
    "пидор",
    "пидар",
    "пидр",
    "педик",
    "гандон",
    "гондон",
    "залуп",
    "дроч",
    "шлюх",
    "долбоеб",
]

ALLOWLIST = frozenset(
    [
        "scunthorpe",
        "grape",
        "grapes",
        "drape",
        "draper",
        "drapes",
        "scrape",
        "scraper",
        "baseball",
        "debate",
        "debater",
        "debates",
        "debating",
        "ashkenazi",
        "swank",
        "swanky",
    ]
)

app = FastAPI()
nsfw_classifier = pipeline("image-classification", model="/model")
concept_classifier = pipeline("zero-shot-image-classification", model="/clip")
en_toxicity = pipeline("text-classification", model="/en-toxicity", top_k=None)
ru_toxicity = pipeline("text-classification", model="/ru-toxicity", top_k=None)
inference_lock = threading.Lock()


def classify(body: bytes):
    try:
        image = Image.open(io.BytesIO(body)).convert("RGB")
    except Exception:
        return None

    with inference_lock:
        nsfw_results = nsfw_classifier(image)
        concept_results = concept_classifier(
            image, candidate_labels=BLOCKED_LABELS + NEUTRAL_LABELS
        )

    nsfw = next((r["score"] for r in nsfw_results if r["label"] == "nsfw"), 0.0)
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


def normalize_text(text: str):
    flat = text.lower().translate(LEET_MAP)
    flat = re.sub(r"[^a-zа-яё]", "", flat)
    collapsed = re.sub(r"(.)\1+", r"\1", flat)
    return flat, collapsed


def transliterate(latin: str) -> str:
    for digraph, cyr in TRANSLIT_DIGRAPHS:
        latin = latin.replace(digraph, cyr)
    return latin.translate(TRANSLIT_SINGLE)


def dictionary_match(text: str):
    flat, collapsed = normalize_text(text)
    plain = re.sub(r"[^a-zа-яё]", "", text.lower())
    if flat in ALLOWLIST or collapsed in ALLOWLIST or plain in ALLOWLIST:
        return None
    cyr_flat = transliterate(flat)
    cyr_collapsed = transliterate(collapsed)
    for root in EN_BLOCKED:
        if root in flat or root in collapsed:
            return root
    for root in RU_BLOCKED:
        if root in flat or root in collapsed or root in cyr_flat or root in cyr_collapsed:
            return root
    return None


def label_scores(raw):
    if isinstance(raw, dict):
        return [raw]
    if isinstance(raw, list):
        if raw and isinstance(raw[0], list):
            raw = raw[0]
        return [r for r in raw if isinstance(r, dict)]
    return []


def toxicity_score(text: str) -> float:
    flat, _ = normalize_text(text)
    cyr = transliterate(flat)
    with inference_lock:
        en_results = label_scores(en_toxicity(text[:512]))
        ru_results = label_scores(ru_toxicity(cyr[:512]))
    en_score = next(
        (r["score"] for r in en_results if r["label"] == "toxic"), 0.0
    )
    ru_score = max(
        (r["score"] for r in ru_results if r["label"] != "non-toxic"),
        default=0.0,
    )
    return max(en_score, ru_score)


def moderate_text(text: str):
    match = dictionary_match(text)
    if match is not None:
        return {"allowed": False, "category": "dictionary", "score": 1.0, "match": match}
    score = toxicity_score(text)
    if score >= TEXT_TOXICITY_THRESHOLD:
        return {"allowed": False, "category": "toxicity", "score": score, "match": None}
    return {"allowed": True, "category": None, "score": score, "match": None}


@app.get("/healthz")
async def healthz():
    return {"ok": True}


@app.post("/check")
async def check(request: Request):
    body = await request.body()
    if not body or len(body) > MAX_BODY_BYTES:
        return Response(status_code=400)
    result = await run_in_threadpool(classify, body)
    if result is None:
        return Response(status_code=400)
    return result


@app.post("/moderate/text")
async def moderate(request: Request):
    try:
        data = await request.json()
    except Exception:
        return Response(status_code=400)
    text = data.get("text") if isinstance(data, dict) else None
    if not isinstance(text, str) or not text or len(text) > TEXT_MAX_LEN:
        return Response(status_code=400)
    return await run_in_threadpool(moderate_text, text)
