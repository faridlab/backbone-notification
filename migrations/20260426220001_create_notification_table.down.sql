-- Down: drop notification.notifications table
DROP TABLE IF EXISTS notification.notifications CASCADE;
DROP FUNCTION IF EXISTS notification.notifications_audit_timestamp() CASCADE;
