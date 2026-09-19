DROP INDEX identity.user_entitlements_member_number_idx;

ALTER TABLE identity.user_entitlements
    DROP COLUMN member_number;

DROP SEQUENCE identity.pro_member_number_seq;
