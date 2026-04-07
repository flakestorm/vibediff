# VibeDiff Backend Cloud

Multi-tenant SaaS backend for paid cloud features.

## Problem Statement

Modern code review pipelines are good at syntax, tests, and static analysis, but weak at semantic intent verification. Teams using AI-assisted coding increasingly see "agentic drift": commits that look valid but diverge from the developer's stated intent. In regulated or high-velocity environments, this creates review noise, weakens trust, and complicates auditability.

The backend-cloud service exists to operationalize semantic auditing at team and enterprise scale: policy-managed scoring, centralized audit trails, org-level controls, and CI-integrated governance.

## Solution Overview

VibeDiff Cloud extends the OSS core by adding organization-aware services:

- authentication, team, and org boundaries
- policy distribution and enforcement
- centralized scoring and audit history
- billing, entitlements, and usage controls
- integrations for CI/PR workflow visibility

The OSS core remains local-first and privacy-preserving. Cloud is additive: it enables governance, collaboration, and operational scale for teams.

## Design Principles

- **Privacy-preserving by default**: cloud endpoints should consume sanitized entity metadata, never raw source code, unless explicitly enabled by enterprise policy.
- **OSS-first compatibility**: cloud must not break local OSS workflows; developers can run fully offline with local models.
- **Deterministic policy behavior**: score thresholds and block/warn logic should be explicit, versioned, and reproducible.
- **Multi-tenant isolation**: strict org/project boundaries for data access, API keys, and audit records.
- **Fail-safe operations**: degraded cloud dependencies should fail predictably with clear fallback semantics.

## Scope
- Auth/token and org management
- Policy APIs
- Scoring APIs (sanitized payloads only)
- Audit/event persistence
- Billing/entitlements

## First Milestones
1. TypeScript backend scaffold
2. Database schema and migrations
3. `/v1/policy/current` and `/v1/score` endpoints
4. Audit ingest/query APIs

## Future Roadmap (Cloud)

### Planned Cloud Version

- Team dashboard for semantic drift trends
- policy editor and policy version history
- PR-level comments and enforcement controls
- org-wide audit timeline and compliance export
- usage metering, seats, and billing management
- enterprise SSO and tenant controls

### How Cloud Differs from OSS

- **OSS (`core`)**: local CLI, local-first checks, optional self-managed integrations
- **Cloud (`backend-cloud` + `frontend-cloud`)**: team collaboration, policy governance, centralized history, billing, and enterprise controls

Cloud is positioned as an operations and governance layer, not a replacement for OSS.

### Waitlist

- Planned waitlist and product updates: [https://vibediff.dev](https://vibediff.dev)
