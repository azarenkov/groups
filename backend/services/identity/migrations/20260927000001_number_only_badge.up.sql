ALTER TABLE identity.users
    DROP CONSTRAINT users_badge_style_check,
    ADD CONSTRAINT users_badge_style_check CHECK (badge_style IN ('', 'tint', 'solid', 'neutral', 'outline', 'ink', 'etched', 'numbered', 'number_only'));
