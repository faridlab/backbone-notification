//! The lifecycle-event outbox mirror must be creatable from this module's own migration chain.
//!
//! `backbone_outbox::outbox::stage` names `company_id` in its INSERT, and the relay reads `org_unit_id` and
//! filters on `failed_at`. The shared test pool heals the table through `outbox::migrate` before any DB test
//! runs, which hides a chain that never creates those columns, so this guard reads the migration files
//! themselves and needs no database.

const CREATE: &str = include_str!("../migrations/20260712000100_notification_outbox.up.sql");
const ADD: &str = include_str!("../migrations/20261113000000_outbox_mirror_framework_columns.up.sql");

/// Columns the framework's outbox code touches on `outbox_events` beyond the module-era base shape.
const FRAMEWORK_COLUMNS: [&str; 4] = ["company_id", "org_unit_id", "failed_at", "failure_reason"];

#[test]
fn the_chain_creates_the_outbox_mirror() {
    assert!(CREATE.contains("notification.outbox_events"), "the outbox mirror is created by the chain");
}

#[test]
fn every_framework_column_is_created_by_the_chain() {
    for column in FRAMEWORK_COLUMNS {
        let created = CREATE.contains(column);
        let added = ADD.contains("ALTER TABLE notification.outbox_events")
            && ADD.contains(&format!("ADD COLUMN IF NOT EXISTS {column}"));
        assert!(
            created || added,
            "the outbox code uses `{column}`: a migration in the chain must create it on notification.outbox_events"
        );
    }
}

#[test]
fn the_added_columns_are_nullable_and_idempotent() {
    // Matches the shape outbox::migrate heals to, so the chain and the heal converge in either order, and a
    // legacy table with rows is never rejected by a NOT NULL add.
    assert!(!ADD.to_uppercase().contains("NOT NULL"), "added columns must stay nullable");
    assert_eq!(
        ADD.matches("ADD COLUMN IF NOT EXISTS").count(),
        FRAMEWORK_COLUMNS.len(),
        "every column is added with IF NOT EXISTS"
    );
}
