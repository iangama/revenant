# M13 — Inspector

Status: complete and validated locally.

The React/Vite Revenant Inspector visually analyzes persisted replay sessions. It provides a recent-session archive, completion and participant metadata, duration and event counts, a searchable chronological event stream, actor labels, and a decoded detail panel for every persisted field and payload.

The existing gateway HTTP listener exposes narrow read-only session and event endpoints backed by typed `revenant-persistence` queries. The production Inspector is a static Nginx container with a same-origin API proxy. The canonical smoke completes a multiplayer activity, restarts the runtime, verifies replay persistence, and requests the session and event APIs. TypeScript checks, production bundling, container health, proxy behavior, and a headless browser rendering are validated locally.

M13 intentionally does not add state mutation, packet injection, raw network capture, compatibility adapters, a frozen client, or M14 work.
