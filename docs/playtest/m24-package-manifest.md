# M24 closed-package manifest

Status: Gate C-approved exact candidate; frozen as a solo engineering artifact, unpublished, and local-only.

## Canonical artifact

- Package ID: `m24-b6-45718926-871f1beafb43`
- Archive:
  `revenant-m24-closed-0.2.0-windows-x86_64-m24-b6-45718926-871f1beafb43.zip`
- Local evidence directory: `/mnt/c/Users/Ian/revenant-local-packages`
- Archive size: `38,728,181` bytes
- Archive SHA-256:
  `8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e`
- Adjacent checksum file: the same archive name with `.sha256` appended.

Only that exact name, package ID, size, and digest may enter Gate C review. The
archive is not a release, is not published, is not code-signed, and is not
authorized for participant use by this manifest alone.

## Build identity

| Field | Frozen value |
| --- | --- |
| Product version | `0.2.0` |
| Branch | `main` |
| Baseline HEAD and `origin/main` | `4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6` |
| Source-tree state | reviewed, intentionally uncommitted |
| Source manifest | 173 runtime/operational inputs |
| Source-manifest SHA-256 | `871f1beafb43a959e3be5a6464f10225b5902d9f8d84ab5ff329442ffe90a279` |
| Godot | `4.7.1.stable.official.a13da4feb` |
| Godot export-template SHA-256 | `86409db6200b6f8fd3230989c2d2002851f3dd18acf11d7bdbafddf5a0dd0f72` |
| Platform | Windows x86-64 |
| Protocol | V2 |
| Reconnect/resume | false |
| Public distribution | false |
| Code signed | false |

The build uses the official
`Godot_v4.7.1-stable_export_templates.tpz` release asset. The Windows preset
keeps `Revenant.pck` separate from the executable and exports release scripts,
without a signing claim.

## Internal artifact hashes

The packaged `SHA256SUMS` is authoritative and is verified by
`operator/Test-RevenantPackage.ps1`. It covers 14 files. The principal binary
hashes are:

- `participant/Revenant.exe`:
  `04baf75cc1d69dd93eb709533ecab4fd7770bb8a530645717017a06a9d9809fc`
- `participant/Revenant.pck`:
  `e8e2c4c9f0e605d3903932b0823173c50346ce8b33ed61a1279655a8aa8978f4`
- `evidence/SOURCE-SHA256SUMS`:
  `871f1beafb43a959e3be5a6464f10225b5902d9f8d84ab5ff329442ffe90a279`

The remaining files are one-page participant instructions, operator runbook,
launcher, checksum verifier, selective report-deletion tool, consent,
observation, intervention, feedback, and evidence-disposition forms, plus the
embedded build identity. The archive contains no repository source tree,
credential, `.env`, log, report, dump, key, or automatic-upload component.

## Reproduction boundary

From the reviewed repository and with the official template archive already
verified under ignored `.tooling`:

```bash
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
  scripts/package-m24-closed-playtest.sh \
  /mnt/c/Users/Ian/revenant-local-packages
```

The script refuses a different version, HEAD, upstream baseline, template
digest, output inside the repository, or existing target. Because the source
tree is intentionally uncommitted, the complete source-input manifest—not HEAD
alone—defines the build.

## Rejected build register

These identifiers are evidence of defects found during package dry-run and are
never eligible for participant use:

| Package ID | Rejection reason |
| --- | --- |
| `m24-b6-45718926-d49372b84a13` | Export omitted raw WAV bytes used by the old runtime loader; Windows validation reported zero decoded audio bytes. |
| `m24-b6-45718926-8891798a9010` | An export include filter did not change imported-resource behavior; rejected before Windows execution. |
| `m24-b6-45718926-be5b67f85ee3` | Game validation passed, but report deletion collided with PowerShell's automatic `$Matches` variable. |
| `m24-b6-45718926-aa49d126b809` | Technically green predecessor, superseded because intervention and evidence-disposition records were not separate package forms. |

The failed pre-promotion build `m24-b6-45718926-6c4c453470f6` never produced a
ZIP and is not an artifact.
