-- Hand-authored (user-owned). Not regenerated.
--
-- Give the lifecycle-event outbox mirror every column the framework's outbox code reads or writes.
--
-- notification.outbox_events mirrors the framework-owned outbox table. The staging INSERT
-- (backbone_outbox::outbox::stage) names company_id; the relay reads company_id and org_unit_id back and
-- filters on failed_at; the dead-letter listing reads failed_at and failure_reason. The create in
-- 20260712000100 predates all four, so on a database built from this migration chain alone every
-- lifecycle-event stage (notify, dispatch, record delivery) failed with "column company_id of relation
-- outbox_events does not exist" until the composing service happened to run backbone_outbox::outbox::migrate
-- on the notification schema.
--
-- company_id is the outbox record's own key, supplied from the ambient org scope; it is not a tenancy fence
-- and is not touched by 20260909120000_strip_tenancy (which strips the notifications and
-- notification_templates tables only).
--
-- Every column is added nullable and IF NOT EXISTS: the exact shape outbox::migrate heals to, so the two paths
-- converge whichever runs first, and a database that already carries a column (healed, or added by the
-- composing service) is untouched. The org-unit fill trigger, the row-level-security fence and the
-- pending-tail index stay owned by outbox::migrate: they are installed by the composer, not by this module.

ALTER TABLE notification.outbox_events
    ADD COLUMN IF NOT EXISTS company_id     uuid,
    ADD COLUMN IF NOT EXISTS org_unit_id    uuid,
    ADD COLUMN IF NOT EXISTS failed_at      timestamptz,
    ADD COLUMN IF NOT EXISTS failure_reason text;
