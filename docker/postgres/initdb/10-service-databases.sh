#!/bin/sh
set -eu

create_service_database() {
    database="$1"
    role="$2"
    password="$3"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
        -v database="$database" -v role="$role" -v password="$password" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'role', :'password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'role') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'database', :'role')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'database') \gexec
SELECT format('REVOKE ALL ON DATABASE %I FROM PUBLIC', :'database') \gexec
SELECT format('GRANT CONNECT, CREATE, TEMPORARY ON DATABASE %I TO %I', :'database', :'role') \gexec
SQL
}

create_service_database "${IDENTITY_POSTGRES_DB:-rift_identity}" "${IDENTITY_POSTGRES_USER:-identity}" "${IDENTITY_POSTGRES_PASSWORD:-identity}"
create_service_database "${MEDIA_POSTGRES_DB:-rift_media}" "${MEDIA_POSTGRES_USER:-media}" "${MEDIA_POSTGRES_PASSWORD:-media}"
create_service_database "${MESSAGING_POSTGRES_DB:-rift_messaging}" "${MESSAGING_POSTGRES_USER:-messaging}" "${MESSAGING_POSTGRES_PASSWORD:-messaging}"
create_service_database "${VOICE_POSTGRES_DB:-rift_voice}" "${VOICE_POSTGRES_USER:-voice}" "${VOICE_POSTGRES_PASSWORD:-voice}"
create_service_database "${REALTIME_POSTGRES_DB:-rift_realtime}" "${REALTIME_POSTGRES_USER:-realtime}" "${REALTIME_POSTGRES_PASSWORD:-realtime}"
