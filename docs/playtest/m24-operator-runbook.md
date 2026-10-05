# M24 closed-playtest operator runbook

Status: unpublished same-host package; no participant session is authorized by
this document alone.

## Hard boundary

- Use only the exact Gate C-reviewed ZIP and package ID.
- Run the Windows client on the operator's trusted Windows machine.
- Keep PostgreSQL, gateway, and Inspector bound to `127.0.0.1`.
- Do not forward ports, change compose bindings, upload the package, publish a
  release, recruit participants, or run unattended.
- Retry always starts from the beginning. There is no reconnect or resume.
- Never delete or recreate the PostgreSQL volume to recover a session.

## Verify the package

1. Compare the ZIP with its adjacent `.zip.sha256` using
   `Get-FileHash -Algorithm SHA256`.
2. Extract into a new empty directory.
3. Run `operator\Test-RevenantPackage.ps1`; every internal file must match
   `SHA256SUMS`.
4. Open `evidence\BUILD-IDENTITY.txt` and copy its `package_id` to the consent,
   observation, and feedback forms.
5. Reject a package whose version, HEAD, source-manifest hash, template hash,
   platform, or `public_distribution=false` fields are absent.

The package is intentionally unsigned. Windows may show an unknown-publisher
warning. Do not weaken antivirus or SmartScreen globally; stop if local policy
does not allow the reviewed file.

## Health gate before every attempt

From the reviewed repository in WSL:

```bash
docker compose -f infra/docker-compose.yml ps
curl --fail http://127.0.0.1:8080/health
ss -ltn | rg '127\.0\.0\.1:(5432|7000|8080|4173)'
```

Require PostgreSQL, gateway, and Inspector to be healthy, with only loopback
published ports. Open `http://127.0.0.1:4173` and confirm the Inspector loads.
Confirm no prior participant client or validator remains active.

Stop before consent if any service is unhealthy, a port is non-loopback, the
package checksum differs, or the environment contains a prior participant's
unresolved report.

## Consent and clean local state

1. Assign a random `PT-[A-Z0-9]{4}` code outside any name/email mapping.
2. Explain the participant sheet and record observation and retention as
   independent choices. Retention cannot be yes when observation is no.
3. Resolve the prior session's report: copy it to controlled evidence only when
   retention was accepted, otherwise delete it.
4. After report disposition, remove only this exact Windows directory to reset
   settings, logs, shader cache, and remaining reports:
   `%APPDATA%\Godot\app_userdata\Revenant`.
5. Reconfirm the exact target before deletion. Never recurse from `%APPDATA%`,
   a profile root, drive root, repository, or unresolved environment variable.

## Start the attempt

From the extracted package:

```powershell
.\operator\Start-Revenant.ps1 `
  -ParticipantCode PT-A1B2 `
  -ObservationConsent yes `
  -RetentionConsent yes
```

The launcher fixes the endpoint to `127.0.0.1:7000`, supplies the package build
ID, and clears its process environment afterward. It never uploads a report.
Give only the participant instruction: “Start Revenant and play until you
believe the session is finished.” Provide no activity coaching.

## Allowed intervention

Intervene only for window/hardware access, a stop request, a crash/freeze,
connection failure, service health, or a milestone stop condition. Record
elapsed time and enumerated reason in `forms\INTERVENTION.md`. Do not
demonstrate input or identify the next objective.

For Retry, first confirm service health. Then launch again; the new attempt is a
fresh activity. Never describe it as resumed.

## Failure and PostgreSQL recovery

1. Stop the participant interaction and preserve the visible outcome.
2. Confirm whether the client socket closed and whether the gateway reset.
3. Inspect `docker compose -f infra/docker-compose.yml ps` and gateway logs.
4. If PostgreSQL is unhealthy, restore it with
   `docker compose -f infra/docker-compose.yml up -d postgres` and wait for
   health. Do not remove its volume.
5. The gateway may reconnect on the next independent operation. If its health
   does not recover after PostgreSQL is healthy, restart only the gateway and
   record that operator action.
6. Use replay/Inspector to classify the old session as incomplete or complete.
7. Require zero ambiguous/partial reward before permitting a fresh Retry.

Stop the pilot on any data loss, duplicate reward, privacy defect, completed
replay without matching reward, reward without matching completion/replay,
participant projection that contradicts persistence, or session that remains
unrecoverable after the bounded procedure.

## Reconciliation after every attempt

1. Immediately record the latest authoritative session ID from Inspector.
2. Review its ordered events and `AuthoritativeSessionSummary`.
3. Locate the local report only when collection was accepted. Protocol V2 does
   not currently populate its optional session ID, so correlate by supervised
   attempt order and record the authoritative ID on the observation sheet.
4. Compare completion, participant/enemy counts, equipment change, loot,
   progression, and durations. Never edit either evidence source to make it
   agree.
5. Classify absence or contradiction as a finding; apply stop conditions where
   integrity or privacy is involved.

## Retention and deletion

- Retention declined: review only during closeout, then delete the report and
  any `.tmp` left by an abrupt exit.
- Retention accepted: copy only the allow-listed report plus structured forms
  to controlled local evidence. Keep scheduling-to-code mappings elsewhere and
  delete that mapping after reconciliation.
- Early deletion: run `operator\Remove-RevenantReport.ps1` with the participant
  code and, when available, report ID. Review its exact target; use `-WhatIf`
  first when uncertain.
- Record every retained, deleted, or never-created item on
  `forms\EVIDENCE-DISPOSITION.md`.
- Delete raw reports and sheets no later than 30 days after Block 8 synthesis.
- After evidence disposition, perform the clean-local-state step before another
  participant.

## Closeout

Confirm participant stop/deletion choices, record whether the attempt is usable,
close the client, reconcile the session, resolve local evidence, reset local
state, and re-run the health gate. Do not begin another session when a stop
condition or unresolved contradiction remains.
