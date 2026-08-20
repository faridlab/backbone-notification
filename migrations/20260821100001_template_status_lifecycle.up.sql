-- Migration: replace the template lifecycle boolean with a status enum
-- notification_templates carried `is_active BOOLEAN NOT NULL DEFAULT TRUE`; the
-- tree-wide convention is one `status` enum field per lifecycle (see
-- docs/refactoring-schema in the serpa workspace). The boolean migrates only
-- rows deviating from its own column default. The enum type is created
-- unqualified so it lands beside the module's other enum types (public), where
-- the generated sqlx type_name resolves.

DO $$ BEGIN
    CREATE TYPE notification_template_status AS ENUM ('active', 'inactive');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

ALTER TABLE notification.notification_templates ADD COLUMN status notification_template_status NOT NULL DEFAULT 'active';
UPDATE notification.notification_templates SET status = 'inactive' WHERE NOT is_active;
ALTER TABLE notification.notification_templates DROP COLUMN is_active;
