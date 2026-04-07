# VibeDiff Team Cloud — Implementation Plan (FastAPI + Next.js)

This document is the **execution plan** for the paid/team **cloud** surfaces. It aligns with [README.md](../README.md) (OSS vs cloud table) and `vibediff_spec.md` (Policy Server, sanitized payloads, enterprise gateway flow).

---

## 1. Repository posture: OSS-first, cloud additive

| Surface | Location | Role |
|--------|----------|------|
| **Public product** | `core/` | **OSS**: Rust CLI, local-first semantic audit, hooks, CI building blocks. This is what the repo **markets** as VibeDiff on GitHub. |
| **Contracts** | `contracts/` | **OSS-shared**: JSON schemas (`AssertionRecord`, sanitized entities) used by CLI **and** cloud so payloads stay compatible. |
| **Team cloud API** | `backend-cloud/` | **Cloud**: Multi-tenant FastAPI service — auth, policy distribution, optional cloud scoring orchestration, audit persistence, entitlements (phased). |
| **Team cloud UI** | `frontend-cloud/` | **Cloud**: Next.js app — onboarding, policy UI, audit dashboards, team settings (consumes `backend-cloud` only). |

**Same monorepo, two audiences**

- **OSS users** clone, install `core`, never run cloud services.
- **Team customers** use hosted `backend-cloud` + `frontend-cloud`; OSS CLI can later opt into cloud via `VIBEDIFF_API_KEY` and policy fetch (per spec §8.3).

Do **not** require cloud for OSS workflows. Cloud features must remain **optional** and **additive**.

---

## 2. OSS vs cloud: responsibility split

Derived from README “OSS vs Cloud” and spec §1.2, §6.3, §8.

### Stays in OSS (`core`)

- Git diff extraction, tree-sitter AST, entity extraction, local cache (sled).
- Local / BYO LLM scoring (Ollama, OpenAI, Anthropic, Gemini, OpenAI-compatible).
- CLI outputs: CLI, JSON, `entity-json`, SARIF.
- Pre-commit hook generation (`vibediff install-hooks`).
- No requirement to call VibeDiff-hosted APIs.

### Lives in cloud (`backend-cloud` + `frontend-cloud`)

| Capability | Spec / README reference |
|------------|-------------------------|
| Org / project / team boundaries | README cloud row; spec multi-tenant |
| OAuth2 / API keys for **team** identity | Spec §8.4; `VIBEDIFF_API_KEY` flow §8.3 |
| Policy Server HTTP API (versioned YAML) | Spec §8.2, §8.3 `GET .../policy/current` |
| Central **audit** storage & query | Spec E5-T6; `POST /api/v1/audit` in §8.3 diagram |
| Optional **cloud score** endpoint (sanitized input only) | Spec §6.3, §8.3 `POST .../score` |
| Dashboards, drift trends, billing UI | README cloud rows |
| GitHub App / PR automation (later phase) | README; spec §6.2 (enterprise-oriented) |

### Privacy rule (non-negotiable)

Cloud APIs accept **sanitized semantic metadata** by default (entity names, paths, change kinds, side-effect types) — see `contracts/v1-sanitized-entity.schema.json` and spec §6.3. **Raw source code is not uploaded** unless a future enterprise-only mode is explicitly designed, documented, and opted in.

---

## 3. Backend: FastAPI (replace TypeScript scaffold)

**Decision:** Implement Team Cloud APIs in **Python [FastAPI](https://fastapi.tiangolo.com/)** under `backend-cloud/`.

**Rationale**

- Aligns with `core/pyproject.toml` / maturin ecosystem and future shared Python utilities.
- Strong OpenAPI generation for the Next.js client and third-party integrations.
- Spec examples and roadmap tasks are transport-oriented; FastAPI maps cleanly to `/api/v1/*`.

**Repo change (planned)**

- Retire or archive the current Node/TypeScript `backend-cloud` scaffold (`package.json`, `src/index.ts`) once FastAPI parity exists, to avoid two backends. Until then, either delete in the same PR as FastAPI MVP or mark TS as deprecated in `backend-cloud/README.md`.

**Suggested layout**

```text
backend-cloud/
  pyproject.toml          # app deps: fastapi, uvicorn, sqlalchemy, pydantic-settings, ...
  README.md               # dev run, env vars, API overview
  app/
    main.py               # FastAPI app factory, CORS, routers
    config.py             # settings from env
    db/                   # SQLAlchemy models, session, migrations (Alembic)
    api/
      v1/
        router.py
        policy.py         # GET /api/v1/policy/current, policy versions
        score.py          # POST /api/v1/score
        audit.py          # POST/GET /api/v1/audit
        health.py         # GET /health, GET /ready
    services/
      policy_service.py
      scoring_service.py  # call external LLM or delegate; validate against contracts
      audit_service.py
    auth/
      dependencies.py     # API key / JWT (phase later: OAuth2 device flow per spec)
    schemas/              # Pydantic models mirroring contracts/
  tests/
```

---

## 4. API surface (v1) — aligned with spec §8.3

Base path: **`/api/v1`**. All responses JSON unless noted.

| Method | Path | Purpose | Auth (phase 1) |
|--------|------|---------|----------------|
| `GET` | `/health` | Liveness | None |
| `GET` | `/ready` | DB / deps readiness | None |
| `GET` | `/api/v1/policy/current` | Active policy document for org/project (YAML or JSON projection) | `Authorization: Bearer <api_key>` or `X-VibeDiff-Key` |
| `GET` | `/api/v1/policy/versions` | List published policy versions (paginated) | Same |
| `POST` | `/api/v1/score` | Accept **sanitized** intent + entities; return `AssertionRecord`-shaped result | Same |
| `POST` | `/api/v1/audit` | Ingest assertion + context for centralized history | Same |
| `GET` | `/api/v1/audit` | Query audit events (filters: repo, time range, label) | Same |

**Request/response shapes**

- Reuse and validate against `contracts/v1-sanitized-entity.schema.json` and `contracts/v1-assertion-record.schema.json` (extend Pydantic models where the Rust CLI emits extra fields: `reasoning`, `flagged_entities`, `suggested_commit_message`).

**Policy payload**

- Phase 1: return JSON that mirrors spec §8.2 YAML structure (thresholds, scope rules) so the CLI can consume one format.
- Phase 2: signed policy bundles and ETag caching as in spec §8.3.

---

## 5. Phased delivery

### Phase A — Skeleton (week 1)

- [ ] Add FastAPI app under `backend-cloud/` with `pyproject.toml`, `uvicorn` entrypoint.
- [ ] Implement `/health`, `/ready`.
- [ ] Wire PostgreSQL (recommended) or SQLite for local dev via SQLAlchemy + Alembic.
- [ ] Stub `GET /api/v1/policy/current` returning a static policy JSON (fixture).
- [ ] Update `.github/workflows/backend-cloud-ci.yml` to run Python (lint, pytest) instead of/in addition to Node.

### Phase B — Audit + score MVP (week 2)

- [ ] `POST /api/v1/audit` — persist minimal audit row (org_id, project_id, assertion JSON, commit_hash, timestamps).
- [ ] `GET /api/v1/audit` — list with pagination and filters.
- [ ] `POST /api/v1/score` — validate input against sanitized schema; call configured LLM provider **server-side** (team keys) or return deterministic mock for tests; output matches assertion contract.
- [ ] Unit tests + OpenAPI snapshot test.

### Phase C — Auth & tenancy (week 3)

- [ ] API key issuance model: `Organization`, `Project`, `ApiKey` tables; hash keys at rest.
- [ ] FastAPI dependency: resolve org/project from key; enforce row-level scope on audit queries.
- [ ] Optional: JWT for **dashboard** users vs API keys for **CI/CLI** (two auth modes).

### Phase D — Frontend (parallel after Phase A)

- [ ] `frontend-cloud`: env `NEXT_PUBLIC_API_BASE_URL` pointing at FastAPI.
- [ ] Pages: login placeholder, policy viewer (read-only), audit list/detail (table + JSON drawer).
- [ ] Generate TypeScript client from OpenAPI (`openapi-typescript` or Orval) in CI or on demand.

### Phase E — CLI integration (later)

- [ ] In `core`, optional `policy_client` + `VIBEDIFF_API_KEY` path: fetch policy, POST score/audit (spec §8.3). **Does not block** cloud MVP.

### Phase F — GitHub App / billing (roadmap)

- [ ] Out of scope for first FastAPI cut; track in README cloud table.

---

## 6. Frontend-cloud (Next.js) integration

| Concern | Approach |
|---------|----------|
| API client | Generated from FastAPI OpenAPI (`/openapi.json`). |
| Auth | Start with API key in server actions or BFF routes; move to OAuth/session for humans. |
| CORS | FastAPI allows `frontend-cloud` dev origin; production uses same-site or explicit allowlist. |
| Deployment | Separate services (e.g. API + Vercel/Cloud Run); document in `docs/CONFIGURATION_GUIDE.md` when stable. |

---

## 7. CI / docs

- [ ] Extend [CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md) with `backend-cloud` env vars (`DATABASE_URL`, `CORS_ORIGINS`, model provider keys for server-side scoring).
- [ ] Update [backend-cloud/README.md](../backend-cloud/README.md) to describe FastAPI, not TypeScript milestones.

---

## 8. Success criteria (Team Cloud v1)

1. FastAPI serves `/api/v1/policy/current`, `/api/v1/score`, `/api/v1/audit` with documented OpenAPI.
2. All cloud write paths require a valid org/project API key (after Phase C).
3. Payloads validate against `contracts/*` for interoperability with OSS `AssertionRecord` semantics.
4. Next.js app reads policy and audit list from the API in a dev environment.
5. OSS `core` remains buildable and usable with **zero** cloud configuration.

---

## 9. References

- [README.md](../README.md) — OSS vs Cloud feature matrix.
- `vibediff_spec.md` — §6.3 Sanitization, §8.2 Policy YAML, §8.3 Enterprise Policy Gateway flow, §8.4 OAuth (future).
- [contracts/README.md](../contracts/README.md) — schema ownership.
