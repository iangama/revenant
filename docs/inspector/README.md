# Revenant Inspector

The M13 Inspector is a read-only engineering interface at `http://127.0.0.1:4173`. It lists the 100 most recent persisted replay sessions and shows activity, completion, participant and event counts, duration, an ordered timeline, actor metadata, and payload detail. M26 adds validated module participant/snapshot/combination/loadout counts and decoded structured before/after evidence. M27 adds only read-only route, resolved-event, terminal outcome/elapsed, transition, and reward-participant facts plus decoded route evidence after independent session reconstruction. M28 adds the reconstructed cooperation state, last phase, terminal outcome/elapsed, participant/contribution/revive/reward counts, and decoded six-kind cooperation evidence. The event filter matches type, actor ID, activity ID, or payload.

The gateway exposes:

- `GET /api/inspector/sessions`
- `GET /api/inspector/sessions/{session-id}/summary`
- `GET /api/inspector/sessions/{session-id}/events`

Session identifiers are restricted to the canonical ASCII alphanumeric-and-hyphen form. Queries are parameterized inside `revenant-persistence`. Structured module, route, and cooperation payloads are bounded and fully validated before the summary or decoded event detail is returned. There are no mutation endpoints, authentication changes, packet injection, route reroll, cooperation mutation, or raw protocol capture.

M30 restricts API requests to the exact configured loopback origin and marks
responses `Cache-Control: no-store`. Direct diagnostic requests must include
`Origin: http://127.0.0.1:4173` (or the configured Inspector port). This is a
browser disclosure boundary, not authentication against another local process.
The Nginx and Vite proxies preserve supplied origins. For browser GETs without
Origin, they derive it from Host only when `Sec-Fetch-Site` is `same-origin`;
Gateway still requires an exact match. Nginx request logging is disabled so
URLs and query values do not enter container logs.

For frontend development, run `npm --prefix web/control-panel run dev`; Vite
binds to `127.0.0.1:4173` and proxies `/api` to the local gateway. For
production-like use, start the Docker Compose stack.
