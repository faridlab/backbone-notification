-- Down: drop the declared rule constraints for notification module
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_body_template_max_length__max_length;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_body_template_required__required;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_subject_template_38cbbe56__max_length;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_name_max_length__max_length;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_name_required__required;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_event_type_max_length__max_length;
ALTER TABLE IF EXISTS notification.notification_templates DROP CONSTRAINT IF EXISTS ck_notification_templates_event_type_required__required;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_failure_reason_max_length__max_length;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_body_max_length__max_length;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_body_required__required;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_subject_max_length__max_length;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_recipient_address_max_length__max_length;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_recipient_address_required__required;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_event_type_max_length__max_length;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_event_type_required__required;
ALTER TABLE IF EXISTS notification.notifications DROP CONSTRAINT IF EXISTS ck_notifications_event_id_required__required;
DO $$
BEGIN
    IF to_regclass('notification.uq_notifications_recipient_address__unique') IS NOT NULL AND to_regclass('notification.idx_notifications_event_id_recipient_address') IS NULL THEN
        ALTER INDEX notification.uq_notifications_recipient_address__unique RENAME TO idx_notifications_event_id_recipient_address;
    END IF;
END $$;

