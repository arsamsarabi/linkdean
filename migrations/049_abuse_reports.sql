-- Abuse reports: signed-in users flag public lists, profiles and comments; admins work the queue.
-- target_id is a category id (public lists), user id (profiles) or comment id, kept as text so
-- the queue still shows a report after its target is gone. target_user_id and target_label are
-- snapshots taken when the report is filed, for the same reason (a deleted comment has no owner
-- to look up, and a disabled public link has no slug).
CREATE TABLE abuse_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK (target_type IN ('public_list', 'profile', 'comment')),
    target_id TEXT NOT NULL,
    target_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    target_label TEXT NOT NULL DEFAULT '',
    reason TEXT NOT NULL,
    details TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    resolution_action TEXT,
    resolution_note TEXT,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX abuse_reports_status_created_idx ON abuse_reports (status, created_at DESC);

-- Where people can reach the operator about abuse or takedowns. NULL until an admin sets it.
ALTER TABLE platform_settings ADD COLUMN abuse_contact_email TEXT;
