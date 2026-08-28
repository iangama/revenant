import { useEffect, useMemo, useState } from "react";

type Session = {
  session_id: string;
  activity_id: string | null;
  started_at: string;
  ended_at: string;
  event_count: number;
  participant_count: number;
  completed: boolean;
};

type ReplayEvent = {
  event_id: number;
  event_type: string;
  timestamp: string;
  session_id: string;
  activity_id: string | null;
  actor_id: number | null;
  payload: string;
};

type AuthoritativeSessionSummary = {
  session_id: string;
  activity_id: string | null;
  first_joined_at: string;
  activity_started_at: string | null;
  activity_ended_at: string;
  join_to_start_ms: number | null;
  activity_duration_ms: number | null;
  participant_count: number;
  completed: boolean;
  enemy_spawn_count: number;
  enemy_defeat_count: number;
  boss_spawned: boolean;
  equipment_change_count: number;
  loot_grant_count: number;
  progression_grant_count: number;
  event_count: number;
};

const API_ROOT = import.meta.env.VITE_INSPECTOR_API ?? "/api/inspector";

function compactId(value: string) {
  return value.length > 28 ? `${value.slice(0, 18)}…${value.slice(-7)}` : value;
}

function time(value: string) {
  return new Date(value).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    fractionalSecondDigits: 3,
  });
}

function duration(milliseconds: number | null | undefined) {
  if (milliseconds == null) return "—";
  return `${(milliseconds / 1000).toFixed(2)}s`;
}

function App() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [events, setEvents] = useState<ReplayEvent[]>([]);
  const [summary, setSummary] = useState<AuthoritativeSessionSummary | null>(null);
  const [selectedEvent, setSelectedEvent] = useState<ReplayEvent | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const loadSessions = async (signal?: AbortSignal, showLoading = true) => {
    if (showLoading) setLoading(true);
    setError("");
    try {
      const response = await fetch(`${API_ROOT}/sessions`, { signal });
      if (!response.ok) throw new Error(`sessions request returned ${response.status}`);
      const body = (await response.json()) as { sessions: Session[] };
      setSessions(body.sessions);
      setSelectedId((current) => current || body.sessions[0]?.session_id || "");
    } catch (caught) {
      if (caught instanceof DOMException && caught.name === "AbortError") return;
      setError(caught instanceof Error ? caught.message : "Inspector API unavailable");
    } finally {
      if (showLoading) setLoading(false);
    }
  };

  useEffect(() => {
    const controller = new AbortController();
    void loadSessions(controller.signal);
    const refresh = window.setInterval(() => {
      void loadSessions(controller.signal, false);
    }, 10_000);
    return () => {
      window.clearInterval(refresh);
      controller.abort();
    };
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setEvents([]);
      setSummary(null);
      return;
    }
    const controller = new AbortController();
    setSelectedEvent(null);
    setSummary(null);
    Promise.all([
      fetch(`${API_ROOT}/sessions/${selectedId}/events`, { signal: controller.signal }),
      fetch(`${API_ROOT}/sessions/${selectedId}/summary`, { signal: controller.signal }),
    ])
      .then(async ([eventsResponse, summaryResponse]) => {
        if (!eventsResponse.ok) throw new Error(`events request returned ${eventsResponse.status}`);
        if (!summaryResponse.ok) throw new Error(`summary request returned ${summaryResponse.status}`);
        const eventBody = (await eventsResponse.json()) as { events: ReplayEvent[] };
        const summaryBody = (await summaryResponse.json()) as { summary: AuthoritativeSessionSummary };
        return { eventBody, summaryBody };
      })
      .then(({ eventBody, summaryBody }) => {
        setEvents(eventBody.events);
        setSummary(summaryBody.summary);
      })
      .catch((caught: unknown) => {
        if (caught instanceof DOMException && caught.name === "AbortError") return;
        setError(caught instanceof Error ? caught.message : "Events unavailable");
      });
    return () => controller.abort();
  }, [selectedId]);

  const selectedSession = sessions.find((session) => session.session_id === selectedId);
  const filteredEvents = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return events;
    return events.filter((event) =>
      [event.event_type, event.payload, event.actor_id?.toString(), event.activity_id]
        .filter(Boolean)
        .some((value) => value!.toLowerCase().includes(normalized)),
    );
  }, [events, query]);

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand-mark" aria-hidden="true"><span /></div>
        <div className="brand-copy">
          <p>REVENANT CORE</p>
          <h1>Session Inspector</h1>
        </div>
        <div className="runtime-status" aria-live="polite"><i /> READ-ONLY / AUTO REFRESH 10S</div>
        <button className="refresh" onClick={() => void loadSessions()} disabled={loading}>
          {loading ? "SYNCING" : "REFRESH"}
        </button>
      </header>

      {error && <div className="error-banner" role="alert">LINK ERROR · {error}</div>}

      <main className="workspace">
        <aside className="sessions-panel">
          <div className="panel-heading">
            <div><span>ARCHIVE</span><h2>Sessions</h2></div>
            <b>{sessions.length.toString().padStart(2, "0")}</b>
          </div>
          <div className="session-list">
            {sessions.map((session, index) => (
              <button
                key={session.session_id}
                className={`session-card ${session.session_id === selectedId ? "active" : ""}`}
                onClick={() => setSelectedId(session.session_id)}
              >
                <div className="session-index">{String(index + 1).padStart(2, "0")}</div>
                <div className="session-copy">
                  <strong title={session.session_id}>{compactId(session.session_id)}</strong>
                  <span>{session.activity_id ?? "unassigned activity"}</span>
                  <small>{new Date(session.started_at).toLocaleString()}</small>
                </div>
                <i className={session.completed ? "complete" : "incomplete"} />
              </button>
            ))}
            {!loading && sessions.length === 0 && <p className="empty">No persisted sessions.</p>}
          </div>
        </aside>

        <section className="timeline-panel">
          {selectedSession ? (
            <>
              <div className="session-header">
                <div>
                  <span className="eyebrow">ACTIVE RECORD</span>
                  <h2>{selectedSession.activity_id}</h2>
                  <code>{selectedSession.session_id}</code>
                </div>
                <div className={`completion ${selectedSession.completed ? "ok" : "pending"}`}>
                  {selectedSession.completed ? "COMPLETED" : "INCOMPLETE"}
                </div>
              </div>

              <div className="metrics">
                <Metric label="EVENTS" value={summary?.event_count ?? selectedSession.event_count} />
                <Metric label="PLAYERS" value={summary?.participant_count ?? selectedSession.participant_count} />
                <Metric label="JOIN → START" value={duration(summary?.join_to_start_ms)} />
                <Metric label="ACTIVITY" value={duration(summary?.activity_duration_ms)} />
                <Metric label="ENEMIES" value={summary ? `${summary.enemy_defeat_count}/${summary.enemy_spawn_count}` : "—"} />
                <Metric label="BOSS" value={summary ? (summary.boss_spawned ? "YES" : "NO") : "—"} />
                <Metric label="LOADOUT" value={summary?.equipment_change_count ?? "—"} />
                <Metric label="REWARDS" value={summary ? `${summary.loot_grant_count}L / ${summary.progression_grant_count}P` : "—"} />
              </div>

              <div className="timeline-toolbar">
                <div><span>EVENT STREAM</span><b>{filteredEvents.length} records</b></div>
                <label>
                  <span>⌕</span>
                  <input
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="Filter type, actor or payload"
                  />
                </label>
              </div>

              <div className="event-stream">
                {filteredEvents.map((event, index) => (
                  <button
                    className={`event-row ${selectedEvent?.event_id === event.event_id ? "selected" : ""}`}
                    key={event.event_id}
                    onClick={() => setSelectedEvent(event)}
                  >
                    <time>{time(event.timestamp)}</time>
                    <div className="rail"><i className={`event-dot type-${event.event_type}`} />{index < filteredEvents.length - 1 && <span />}</div>
                    <div className="event-copy">
                      <strong>{event.event_type.replaceAll("_", " ")}</strong>
                      <p>{event.payload}</p>
                    </div>
                    <div className="actor-tag">{event.actor_id ? `ACTOR ${event.actor_id}` : "SYSTEM"}</div>
                    <span className="arrow">›</span>
                  </button>
                ))}
              </div>
            </>
          ) : (
            <div className="empty-state"><div className="brand-mark"><span /></div><h2>Select a session</h2><p>Persisted replay events will appear here.</p></div>
          )}
        </section>

        <aside className={`detail-panel ${selectedEvent ? "open" : ""}`}>
          <div className="panel-heading"><div><span>DECODED</span><h2>Event detail</h2></div>{selectedEvent && <button onClick={() => setSelectedEvent(null)}>×</button>}</div>
          {selectedEvent ? (
            <div className="detail-content">
              <div className="event-id">EVENT / {selectedEvent.event_id.toString().padStart(6, "0")}</div>
              <h3>{selectedEvent.event_type.replaceAll("_", " ")}</h3>
              <Detail label="TIMESTAMP" value={selectedEvent.timestamp} />
              <Detail label="SESSION" value={selectedEvent.session_id} mono />
              <Detail label="ACTIVITY" value={selectedEvent.activity_id ?? "—"} />
              <Detail label="ACTOR" value={selectedEvent.actor_id?.toString() ?? "—"} />
              <div className="payload"><span>PAYLOAD</span><pre>{selectedEvent.payload}</pre></div>
              <div className="evidence"><i /> PERSISTED / CONFIRMED</div>
            </div>
          ) : (
            <div className="detail-empty"><div>+</div><p>Select an event to inspect its persisted fields.</p></div>
          )}
        </aside>
      </main>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return <div className="metric"><span>{label}</span><strong>{value}</strong></div>;
}

function Detail({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div className="detail-line"><span>{label}</span><p className={mono ? "mono" : ""}>{value}</p></div>;
}

export default App;
