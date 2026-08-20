# M3 — World Join

Status: complete and validated locally.

The authenticated client selects its server-provided character. The runtime validates ownership, joins `relay-hub`, allocates a server-owned player actor ID, and returns spawn position `[0, 0, 0]`. No enemy actors, movement, or replication are part of M3.
