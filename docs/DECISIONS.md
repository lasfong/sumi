# Technical Decisions

This document records technical decisions made during the development of Sumi.

## Decision 1: Database Choice
- **Decision**: Use SQLite for the first version, abstracted via SQLAlchemy.
- **Reason**: Meets the local-first requirement without requiring a complex setup like PostgreSQL for individual users.

## Decision 2: Project Structure
- **Decision**: Separate models, schemas, services, and domain errors/enums.
- **Reason**: Ensures business logic is not tangled with API routes or database models.

## Decision 3: Replay Rule - No Future Data Leak
- **Decision**: Backend API will only serve candles up to the `current_index` of the active ReplaySession.
- **Reason**: Strictly enforces that the frontend cannot access future data and thereby cheat or leak future insights.
