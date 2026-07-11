-- Durable staging for notification's delivery-state events (outbox rollout plan, P2). A consumer subscribes
-- to NotificationDelivered/NotificationUndelivered for delivery-based escalation; a crash between the status
-- CAS and the in-proc publish would drop the signal (escalation never fires). Staging in the same tx as the
-- status transition makes it survive. Standard 11-column outbox shape.
CREATE TABLE IF NOT EXISTS notification.outbox_events (
  id uuid PRIMARY KEY, event_type text NOT NULL, aggregate_type text NOT NULL, aggregate_id text NOT NULL,
  payload jsonb NOT NULL, occurred_at timestamptz NOT NULL, correlation_id text, causation_id text,
  version int NOT NULL DEFAULT 1, created_at timestamptz NOT NULL DEFAULT now(), published_at timestamptz );
CREATE INDEX IF NOT EXISTS idx_notification_outbox_unpublished ON notification.outbox_events (occurred_at) WHERE published_at IS NULL;
CREATE TABLE IF NOT EXISTS notification.inbox_consumed (
  consumer text NOT NULL, event_id uuid NOT NULL, consumed_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY (consumer, event_id) );
