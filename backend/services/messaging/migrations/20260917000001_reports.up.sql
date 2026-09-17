CREATE TABLE messaging.reports (
    id uuid NOT NULL,
    reporter_id uuid NOT NULL,
    target_type character varying(10) NOT NULL,
    target_id uuid NOT NULL,
    target_name character varying(100) NOT NULL,
    reason character varying(20) NOT NULL,
    details character varying(1000),
    context_channel_id uuid,
    context_message_id uuid,
    status character varying(10) DEFAULT 'open' NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    reviewed_at timestamp with time zone,
    CONSTRAINT reports_pkey PRIMARY KEY (id),
    CONSTRAINT reports_target_type_check CHECK (target_type IN ('user', 'server')),
    CONSTRAINT reports_status_check CHECK (status IN ('open', 'reviewed', 'rejected')),
    CONSTRAINT reports_reason_check CHECK (reason IN ('spam', 'harassment', 'hate', 'threats', 'scam', 'nsfw', 'csam', 'self_harm', 'impersonation', 'illegal', 'violence', 'raid', 'other')),
    CONSTRAINT reports_details_not_blank CHECK (details IS NULL OR length(btrim(details)) > 0),
    CONSTRAINT reports_no_self_report CHECK (target_type <> 'user' OR reporter_id <> target_id)
);

CREATE INDEX reports_target_idx ON messaging.reports (target_type, target_id, created_at DESC);

CREATE INDEX reports_reporter_idx ON messaging.reports (reporter_id, created_at DESC);

CREATE INDEX reports_open_idx ON messaging.reports (created_at DESC) WHERE status = 'open';
