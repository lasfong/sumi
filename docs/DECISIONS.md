# Technical Decisions

This document records technical decisions made during the development of Sumi.

## Decision 0: Canonical Product Spec
- **Decision**: `docs/PRODUCT_SPEC.md` is the canonical implementation spec for the current MVP.
- **Reason**: The repository currently implements the local-first manual replay and journal product described there. `docs/SPEC.md` contains a broader long-term vision and must not be treated as the immediate delivery contract unless a future decision promotes specific sections.
- **Implication**: When `SPEC.md` conflicts with `PRODUCT_SPEC.md`, follow `PRODUCT_SPEC.md` and this decisions file for current development.

## Decision 1: Database Choice
- **Decision**: Use SQLite for the first version, abstracted via SQLAlchemy.
- **Reason**: Meets the local-first requirement without requiring a complex setup like PostgreSQL for individual users.
- **Implication**: PostgreSQL/TimescaleDB remains future scope. Do not add it until the MVP is stable and a separate migration plan exists.

## Decision 2: Project Structure
- **Decision**: Separate models, schemas, services, and domain errors/enums.
- **Reason**: Ensures business logic is not tangled with API routes or database models.

## Decision 3: Replay Rule - No Future Data Leak
- **Decision**: Backend API will only serve candles up to the `current_index` of the active ReplaySession.
- **Reason**: Strictly enforces that the frontend cannot access future data and thereby cheat or leak future insights.

## Decision 4: Feature Priority
- **Decision**: Stabilize manual replay, trade lifecycle, and analytics before building the algorithmic strategy engine.
- **Reason**: Current audit found frontend build failures and incomplete/noisy backend tests. Adding strategy automation before the core is reliable would create more unstable surface area.
- **Implementation Order**: Follow `docs/IMPLEMENTATION_PLAYBOOK.md`.

## Decision 5: Data and POC Artifacts
- **Decision**: Raw CafeF files, local databases, generated caches, and research repo clones are local artifacts, not project documentation.
- **Reason**: Large raw data and POC clones make the repo noisy and confuse the source of truth.
- **Implication**: Raw data belongs under `data/raw/` locally and is ignored by git. Research clones belong under `research_repos/` locally and are ignored by git.
