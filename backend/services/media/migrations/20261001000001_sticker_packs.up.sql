CREATE TABLE media.media_sticker_packs (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    name character varying(64) NOT NULL,
    slug character varying(32) NOT NULL,
    install_count integer DEFAULT 0 NOT NULL,
    hidden boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT media_sticker_packs_pkey PRIMARY KEY (id),
    CONSTRAINT media_sticker_packs_slug_check CHECK (slug ~ '^[a-z0-9_]{3,32}$'),
    CONSTRAINT media_sticker_packs_install_count_check CHECK (install_count >= 0)
);

CREATE UNIQUE INDEX media_sticker_packs_slug_idx ON media.media_sticker_packs (slug) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX media_sticker_packs_name_idx ON media.media_sticker_packs (lower(name)) WHERE deleted_at IS NULL;
CREATE INDEX media_sticker_packs_owner_idx ON media.media_sticker_packs (owner_id) WHERE deleted_at IS NULL;
CREATE INDEX media_sticker_packs_search_idx ON media.media_sticker_packs (install_count DESC, id) WHERE deleted_at IS NULL AND NOT hidden;

CREATE TABLE media.media_stickers (
    id uuid NOT NULL,
    pack_id uuid NOT NULL,
    storage_key text NOT NULL,
    animated boolean NOT NULL,
    width integer NOT NULL,
    height integer NOT NULL,
    emoji character varying(32) NOT NULL,
    position integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT media_stickers_pkey PRIMARY KEY (id),
    CONSTRAINT media_stickers_pack_id_fkey FOREIGN KEY (pack_id) REFERENCES media.media_sticker_packs(id) ON DELETE CASCADE,
    CONSTRAINT media_stickers_size_check CHECK (width > 0 AND height > 0)
);

CREATE INDEX media_stickers_pack_idx ON media.media_stickers (pack_id, position) WHERE deleted_at IS NULL;

CREATE TABLE media.media_sticker_installs (
    user_id uuid NOT NULL,
    pack_id uuid NOT NULL,
    installed_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT media_sticker_installs_pkey PRIMARY KEY (user_id, pack_id),
    CONSTRAINT media_sticker_installs_pack_id_fkey FOREIGN KEY (pack_id) REFERENCES media.media_sticker_packs(id) ON DELETE CASCADE
);
