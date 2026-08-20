-- Down: restore the is_active boolean exactly as it was.
-- Only 'inactive' rows are written back as FALSE; rows at the column default
-- map to the boolean default TRUE without an UPDATE.

ALTER TABLE notification.notification_templates ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE;
UPDATE notification.notification_templates SET is_active = FALSE WHERE status = 'inactive';
ALTER TABLE notification.notification_templates DROP COLUMN status;
DROP TYPE IF EXISTS notification_template_status;
