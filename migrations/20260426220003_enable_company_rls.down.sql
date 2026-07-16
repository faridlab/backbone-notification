-- Down: remove the company RLS fence for notification module

-- Reverse the company RLS fence for notification.notifications
DROP POLICY IF EXISTS notifications_company_isolation ON notification.notifications;
ALTER TABLE notification.notifications NO FORCE ROW LEVEL SECURITY;
ALTER TABLE notification.notifications DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for notification.notification_templates
DROP POLICY IF EXISTS notification_templates_company_isolation ON notification.notification_templates;
ALTER TABLE notification.notification_templates NO FORCE ROW LEVEL SECURITY;
ALTER TABLE notification.notification_templates DISABLE ROW LEVEL SECURITY;

