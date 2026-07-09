-- Down: drop notification.notification_templates table
DROP TABLE IF EXISTS notification.notification_templates CASCADE;
DROP FUNCTION IF EXISTS notification.notification_templates_audit_timestamp() CASCADE;
