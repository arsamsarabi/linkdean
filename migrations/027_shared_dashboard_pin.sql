-- An invitee can pin a shared dashboard as their default. The pin lives on the
-- share row, so losing access removes it and their own default applies again.

ALTER TABLE dashboard_shares
    ADD COLUMN is_pinned BOOLEAN NOT NULL DEFAULT FALSE;

CREATE UNIQUE INDEX dashboard_shares_one_pin_per_invitee
    ON dashboard_shares (invitee_user_id)
    WHERE is_pinned;
