-- Hand-authored (user-owned). Not regenerated.
--
-- Drops the columns the up migration added. Reversible but not durable: the framework's outbox::migrate adds
-- the same columns again on the next boot of any service that runs it for the notification schema, and any
-- value already stored in them is lost here.

ALTER TABLE notification.outbox_events
    DROP COLUMN IF EXISTS failure_reason,
    DROP COLUMN IF EXISTS failed_at,
    DROP COLUMN IF EXISTS org_unit_id,
    DROP COLUMN IF EXISTS company_id;
