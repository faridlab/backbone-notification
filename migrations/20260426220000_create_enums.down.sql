-- Down: drop enum types for notification module
DROP TYPE IF EXISTS notif_channel CASCADE;
DROP TYPE IF EXISTS notification_status CASCADE;
