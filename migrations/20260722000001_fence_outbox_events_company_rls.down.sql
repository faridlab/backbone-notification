DROP POLICY IF EXISTS outbox_events_company_isolation ON notification.outbox_events;
ALTER TABLE notification.outbox_events NO FORCE ROW LEVEL SECURITY;
ALTER TABLE notification.outbox_events DISABLE ROW LEVEL SECURITY;
DROP INDEX IF EXISTS notification.idx_notification_outbox_company_id;
ALTER TABLE notification.outbox_events DROP COLUMN IF EXISTS company_id;
