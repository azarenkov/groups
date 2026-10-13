DO $$
DECLARE
    trgm_schema text;
BEGIN
    SELECT n.nspname INTO STRICT trgm_schema
    FROM pg_extension e
    JOIN pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'pg_trgm';

    EXECUTE format('CREATE INDEX idx_messaging_user_profiles_name_trgm ON messaging.messaging_user_profiles USING gin (lower(name) %I.gin_trgm_ops)', trgm_schema);
END
$$;
