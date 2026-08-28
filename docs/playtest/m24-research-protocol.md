# M24 closed-playtest research protocol

Status: draft for Block 1 review; no participant session is authorized by this document.

## Session format

Target duration: 25–35 minutes per participant.

1. **Introduction — 3 minutes:** explain that the software, not the participant, is being tested; identify what will be observed; confirm consent; assign a non-identifying participant code.
2. **Unassisted first contact — up to 15 minutes:** provide the package and the single instruction “Start Revenant and play until you believe the session is finished.” Do not explain controls, objectives, enemies, equipment, or the door.
3. **Second attempt if needed — up to 5 minutes:** allow a clean retry without coaching when failure is recoverable.
4. **Interview — 7–10 minutes:** ask the fixed questions below before discussing implementation.
5. **Closeout — 2 minutes:** explain saved evidence, offer deletion, record whether the local report may be retained, and reset the environment.

## Moderator rules

- Do not name the next objective or demonstrate a control.
- Answer hardware, window, and crash-recovery questions only.
- Mark each intervention with time and reason.
- Record direct observation separately from inference.
- Do not convert feature suggestions into promises.
- Stop immediately on request or on a milestone stop condition.

## Observation sheet

| Field | Allowed values |
| --- | --- |
| Participant code | Random test code, no name or email |
| Environment | OS, resolution, window/fullscreen, input device |
| Connect outcome | Unassisted / assisted / failed |
| First movement | Time and whether guidance was used |
| First attack | Time and whether target selection was understood |
| Equipment | Not noticed / inspected / changed |
| Door transition | Understood / discovered accidentally / blocked |
| Warden | Understood / unclear / blocked |
| Completion | Unassisted / assisted / incomplete |
| Interventions | Timestamp plus enumerated reason |
| Blocking defect | Identifier only; detail goes into the reviewed finding |

Free-form notes must describe visible behavior, not personal characteristics. Do not record unrelated conversation.

## Fixed interview questions

1. What did you think your goal was?
2. Which moment was most confusing?
3. How did you know an attack had worked or failed?
4. What changed when you selected another weapon?
5. What did you believe the door required?
6. What reward or progress did you receive?
7. What would you most want to do next?
8. Would you choose to play another activity now? Why or why not?

## Finding classification

Each finding receives:

- category: comprehension, interaction, gameplay/content, reliability, environment, or privacy;
- evidence source: observation, local report, server summary, participant statement, or moderator intervention;
- frequency: number of usable sessions affected;
- severity: cosmetic, friction, task failure, session failure, data/integrity risk, or privacy risk;
- confidence: low, medium, or high;
- recommended disposition: fix in M24, candidate next milestone, accept, or investigate.

Only playtest-invalidating or safety/integrity defects may be fixed during the pilot. Other findings wait for Block 8 synthesis.

## Consent and retention baseline

- Participation is voluntary and may stop at any time.
- Local observation export is opt-in and shown before the session.
- No automatic upload occurs.
- Raw evidence is retained for at most 30 days after synthesis unless the participant requests earlier deletion.
- The participant code mapping, if any is needed for scheduling, is stored separately and deleted after the session is reconciled.
- Findings retained in project documentation contain no participant identity.

This is a project research protocol, not legal advice or a substitute for any consent, privacy, employment, or research requirements applicable to the actual recruitment context.

## Decision rule

Block 8 chooses one primary next direction:

- **Content:** comprehension and reliability are adequate, but replay desire is limited by breadth.
- **Interaction/accessibility:** controls, layout, focus, readability, or settings dominate failure and friction.
- **Resilience/security:** connection, recovery, session lifecycle, or operational defects invalidate otherwise useful play.
- **Core revision:** participants understand the slice but do not find its central activity compelling.

The decision record must state why the other three directions were not selected as the next milestone.
