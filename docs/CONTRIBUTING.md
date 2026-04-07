# Contributing to VibeDiff

## Development Setup

### Core

```bash
cd core
cargo test
```

### Backend Cloud

```bash
cd backend-cloud
npm install
npm run build
```

### Frontend Cloud

```bash
cd frontend-cloud
npm install
npm run build
```

## Contribution Rules

- Keep modules small and composable.
- Add tests for functional changes.
- Do not mix unrelated refactors in one PR.
- Update `IMPLEMENTATION_TRACKER.md` for major phase changes.

## PR Checklist

- [ ] Code builds/tests pass locally
- [ ] Docs updated if behavior changed
- [ ] Contracts updated if API shape changed
- [ ] Tracker updated
