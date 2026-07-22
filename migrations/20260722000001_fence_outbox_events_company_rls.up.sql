-- ADR-0011: fence notification.outbox_events by company_id (extracted from the payload).
ALTER TABLE notification.outbox_events ADD COLUMN IF NOT EXISTS company_id UUID;
UPDATE notification.outbox_events SET company_id = (payload ->> 'company_id')::uuid WHERE company_id IS NULL;
ALTER TABLE notification.outbox_events ALTER COLUMN company_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notification_outbox_company_id ON notification.outbox_events (company_id);
ALTER TABLE notification.outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification.outbox_events FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS outbox_events_company_isolation ON notification.outbox_events;
CREATE POLICY outbox_events_company_isolation ON notification.outbox_events
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
