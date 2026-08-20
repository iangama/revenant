# Revenant Inspector

The M13 Inspector is a read-only engineering interface at `http://127.0.0.1:4173`. It lists the 100 most recent persisted replay sessions and shows activity, completion, participant and event counts, duration, an ordered timeline, actor metadata, and payload detail. The event filter matches type, actor ID, activity ID, or payload.

The gateway exposes:

- `GET /api/inspector/sessions`
- `GET /api/inspector/sessions/{session-id}/events`

Session identifiers are restricted to the canonical ASCII alphanumeric-and-hyphen form. Queries are parameterized inside `revenant-persistence`. There are no mutation endpoints, authentication changes, packet injection, or raw protocol capture in M13.

For frontend development, run `npm --prefix web/control-panel run dev`; Vite proxies `/api` to the local gateway. For production-like use, start the Docker Compose stack.
