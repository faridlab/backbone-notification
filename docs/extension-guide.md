# backbone-notification — Extension Guide

## Public surface (stable)
- **Dispatch port** (`application::service::notification_ports`): `CommunicationPort` + DTOs
  (`DispatchRequest`, `DispatchAck`, `DispatchRejected`) — the seam a composing service implements over
  backbone-communication's `send_outbound`. Notification never imports communication.
- **Write path** (`application::service::notification_write_service::NotificationWriteService`):
  `create_template`, `notify` (the fan-out engine), `dispatch_pending` (the retry reaper), plus DTOs
  (`NewTemplate`, `NotifyEvent`, `Recipient`, `NotifyOutcome`, `NotifyError`).
- **Events** (`application::service::notification_events`): `NotificationDispatched`,
  `NotificationFailed`, the `NotificationEvent` union, and `NotificationEventSink` — a terminal
  observability surface.

## How a consuming service uses notification
Author templates per (event_type, channel) with `{{placeholder}}` bodies. On a domain event, call
`notify(NotifyEvent { event_id, event_type, channel, recipients, data }, port, sink)` — pass the event's
stable id as `event_id` so redelivery dedups. Implement `CommunicationPort::dispatch` over
backbone-communication, forwarding `DispatchRequest.idempotency_key` as a provider-level dedup token so a
retry never double-sends. Run `dispatch_pending` on a schedule to recover notifications left `pending` by
a crash between the slot claim and the dispatch.

## Not a contract
- The 12 generated CRUD endpoints per entity are convenience scaffolding. Do **not** insert a notification
  or flip a status through the generic PATCH surface — it bypasses the (event_id, recipient) dedup and the
  dispatch/retry gating. Use `NotificationWriteService`.
- `// <<< CUSTOM` blocks preserve local edits only; not a cross-module extension point.

## Invariants a consumer must not break
- One notification per `(event_id, recipient_address)`; a redelivered event never double-notifies.
- A notification is dispatched at most once *to the recipient* when the `CommunicationPort` honors the
  idempotency key; `dispatch_pending` re-drives only rows not yet `sent`.
- Rendering is flat `{{placeholder}}` substitution — templates carry no logic.
