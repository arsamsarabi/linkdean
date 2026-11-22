-- Stripe customer id for managed Pro checkout (Plans Phase C). Self-host leaves NULL.
ALTER TABLE users
    ADD COLUMN stripe_customer_id TEXT;
