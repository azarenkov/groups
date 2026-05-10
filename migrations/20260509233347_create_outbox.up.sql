CREATE TYPE aggregate_type AS ENUM (
    'channel',
    'server',
    'user'
);

CREATE TABLE outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Агрегат, к которому относится событие
    aggregate_id    UUID NOT NULL,
    aggregate_type  aggregate_type NOT NULL,

    -- Родительский агрегат (например, server для channel).
    -- Используется для построения иерархических NATS subjects.
    parent_id       UUID,
    parent_type     aggregate_type,

    -- Тип события (например, "created", "participant.joined").
    -- Не enum, потому что список открыт и быстро растёт.
    event_type      VARCHAR(128) NOT NULL,

    -- Сериализованное событие (JSON).
    payload         JSONB NOT NULL,

    -- Когда событие произошло в домене (не путать с created_at).
    occurred_at     TIMESTAMPTZ NOT NULL,

    -- Когда запись попала в outbox.
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Когда воркер успешно опубликовал в NATS. NULL = ещё не обработано.
    processed_at    TIMESTAMPTZ,

    -- Счётчик попыток публикации.
    attempts        SMALLINT NOT NULL DEFAULT 0,

    -- Текст последней ошибки публикации, для диагностики.
    last_error      TEXT,

    -- Если parent_id есть, parent_type обязателен, и наоборот.
    -- Защита от рассинхронизации полей.
    CONSTRAINT outbox_parent_consistency CHECK (
        (parent_id IS NULL AND parent_type IS NULL)
        OR (parent_id IS NOT NULL AND parent_type IS NOT NULL)
    ),

    -- Граница для счётчика попыток.
    CONSTRAINT outbox_attempts_check CHECK (attempts >= 0 AND attempts <= 100)
);

-- Главный индекс для воркера: быстро находить необработанные события.
-- Partial index — только по строкам, которые ещё не обработаны.
CREATE INDEX outbox_unprocessed_idx
    ON outbox (created_at)
    WHERE processed_at IS NULL;

-- Для аудита и отладки: события по конкретному агрегату.
CREATE INDEX outbox_aggregate_idx
    ON outbox (aggregate_type, aggregate_id, created_at);

-- Опционально: для очистки старых обработанных записей.
CREATE INDEX outbox_processed_idx
    ON outbox (processed_at)
    WHERE processed_at IS NOT NULL;
