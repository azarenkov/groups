FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir torch --index-url https://download.pytorch.org/whl/cpu \
    && pip install --no-cache-dir transformers pillow fastapi uvicorn

RUN python -c "from huggingface_hub import snapshot_download; snapshot_download('Falconsai/nsfw_image_detection', local_dir='/model')"

RUN python -c "from huggingface_hub import snapshot_download; snapshot_download('openai/clip-vit-base-patch32', local_dir='/clip')"

RUN python -c "from huggingface_hub import snapshot_download; snapshot_download('martin-ha/toxic-comment-model', local_dir='/en-toxicity')"

RUN python -c "from huggingface_hub import snapshot_download; snapshot_download('cointegrated/rubert-tiny-toxicity', local_dir='/ru-toxicity')"

ENV HF_HUB_OFFLINE=1 \
    TRANSFORMERS_OFFLINE=1

COPY backend/nsfw-detector/app.py /app/app.py

EXPOSE 8000

CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8000"]
