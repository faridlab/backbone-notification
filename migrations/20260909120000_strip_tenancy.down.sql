-- Hand-authored (user-owned). Not regenerated.
--
-- Best-effort restore sketch for the tenancy strip (ADR-0029). This is a breaking module
-- release against dev-stage databases: the down re-adds the company_id column as nullable
-- with its plain index and the company isolation policy shape, but restores NO data —
-- rows written after the strip (or after the decorator re-keyed them) carry org_unit_id
-- only. The composing service's tenancy decorator remains the live fence; treat this
-- down as a schema-shape sketch for archaeology, not a usable rollback.

ALTER TABLE notification.notifications         ADD COLUMN IF NOT EXISTS company_id uuid;
ALTER TABLE notification.notification_templates ADD COLUMN IF NOT EXISTS company_id uuid;

CREATE INDEX IF NOT EXISTS idx_notifications_company_id_status
    ON notification.notifications (company_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_templates_company_id_event_type_channel
    ON notification.notification_templates (company_id, event_type, channel);

-- The company isolation policies return in the ADR-0008 shape. RLS may not be
-- enabled/forced on these tables any more (the decorator owns those flags), so enable
-- first — idempotent either way.
ALTER TABLE notification.notifications         ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification.notifications         FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS notifications_company_isolation ON notification.notifications;
CREATE POLICY notifications_company_isolation ON notification.notifications
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE notification.notification_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification.notification_templates FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS notification_templates_company_isolation ON notification.notification_templates;
CREATE POLICY notification_templates_company_isolation ON notification.notification_templates
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);
