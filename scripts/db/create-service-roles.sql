\set identity_password `echo "${IDENTITY_DB_PASSWORD:-identity-secret}"`
\set media_password `echo "${MEDIA_DB_PASSWORD:-media-secret}"`
\set messaging_password `echo "${MESSAGING_DB_PASSWORD:-messaging-secret}"`
\set voice_password `echo "${VOICE_DB_PASSWORD:-voice-secret}"`
\set realtime_password `echo "${REALTIME_DB_PASSWORD:-realtime-secret}"`

SELECT format('CREATE ROLE identity_svc LOGIN PASSWORD %L', :'identity_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'identity_svc') \gexec
SELECT format('CREATE ROLE media_svc LOGIN PASSWORD %L', :'media_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'media_svc') \gexec
SELECT format('CREATE ROLE messaging_svc LOGIN PASSWORD %L', :'messaging_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'messaging_svc') \gexec
SELECT format('CREATE ROLE voice_svc LOGIN PASSWORD %L', :'voice_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'voice_svc') \gexec
SELECT format('CREATE ROLE realtime_svc LOGIN PASSWORD %L', :'realtime_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'realtime_svc') \gexec

GRANT USAGE ON SCHEMA identity TO identity_svc;
GRANT ALL ON ALL TABLES IN SCHEMA identity TO identity_svc;
ALTER DEFAULT PRIVILEGES IN SCHEMA identity GRANT ALL ON TABLES TO identity_svc;

GRANT USAGE ON SCHEMA media TO media_svc;
GRANT ALL ON ALL TABLES IN SCHEMA media TO media_svc;
ALTER DEFAULT PRIVILEGES IN SCHEMA media GRANT ALL ON TABLES TO media_svc;

GRANT USAGE ON SCHEMA messaging TO messaging_svc;
GRANT ALL ON ALL TABLES IN SCHEMA messaging TO messaging_svc;
ALTER DEFAULT PRIVILEGES IN SCHEMA messaging GRANT ALL ON TABLES TO messaging_svc;

GRANT USAGE ON SCHEMA voice TO voice_svc;
GRANT ALL ON ALL TABLES IN SCHEMA voice TO voice_svc;
ALTER DEFAULT PRIVILEGES IN SCHEMA voice GRANT ALL ON TABLES TO voice_svc;

GRANT USAGE ON SCHEMA realtime TO realtime_svc;
GRANT ALL ON ALL TABLES IN SCHEMA realtime TO realtime_svc;
ALTER DEFAULT PRIVILEGES IN SCHEMA realtime GRANT ALL ON TABLES TO realtime_svc;

GRANT USAGE ON SCHEMA public TO identity_svc, media_svc, messaging_svc, voice_svc, realtime_svc;
GRANT ALL ON public.outbox TO identity_svc, media_svc, messaging_svc, voice_svc, realtime_svc;
GRANT ALL ON public.storage_cleanup_queue TO media_svc, messaging_svc;

ALTER ROLE identity_svc IN DATABASE rift SET search_path TO identity, public;
ALTER ROLE media_svc IN DATABASE rift SET search_path TO media, public;
ALTER ROLE messaging_svc IN DATABASE rift SET search_path TO messaging, public;
ALTER ROLE voice_svc IN DATABASE rift SET search_path TO voice, public;
ALTER ROLE realtime_svc IN DATABASE rift SET search_path TO realtime, public;
