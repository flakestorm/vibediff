# Shared Contracts (v1)

This directory is the canonical interface boundary between:

- `core` (Rust CLI/engine)
- `backend-cloud` (SaaS API)
- `frontend-cloud` (Next.js)

Versioning rules:

- Additive fields are allowed in `v1`.
- Breaking changes require `v2` schemas and dual-read migration windows.
- Core should ignore unknown fields from backend responses.
