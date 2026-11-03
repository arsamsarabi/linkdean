-- Emails waiting to be sent, and a short history of sent and failed ones.
-- Bodies are rendered at enqueue time, so a later template change never alters a queued email.
CREATE TABLE email_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipient TEXT NOT NULL,
    template TEXT NOT NULL,
    subject TEXT NOT NULL,
    html TEXT NOT NULL,
    text TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'sent', 'failed')),
    attempts INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A sender holds a row until this time; a crashed sender's rows come back after it.
    claimed_until TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX email_outbox_due ON email_outbox (next_attempt_at) WHERE status = 'queued';
CREATE INDEX email_outbox_status_updated ON email_outbox (status, updated_at);
