"""Unit and integration tests for the managed universe domain, resolver, candidate artifact, and API."""

from datetime import date
import pytest
from fastapi.testclient import TestClient

from app.domain.universe.models import (
    CoverageStatus,
    MemberStatus,
    UniverseCoverageReport,
    UniverseDefinition,
    UniverseMember,
    UniverseMode,
    UniverseResolution,
    UniverseStatus,
)
from app.domain.universe.registry import UniverseRegistry, get_universe_registry
from app.domain.universe.resolver import UniverseResolver
from app.main import app


# ---------------------------------------------------------------------------
# 1. Domain Models & Content Hash Determinism
# ---------------------------------------------------------------------------

def test_universe_model_and_hash_determinism():
    """Verify that UniverseDefinition.content_hash() is deterministic and insensitive to member insertion order."""
    m1 = UniverseMember(
        symbol="FPT",
        exchange="HOSE",
        effective_from=date(2024, 1, 1),
        effective_to=None,
        status=MemberStatus.ACTIVE,
        weight=1.0,
    )
    m2 = UniverseMember(
        symbol="HPG",
        exchange="HOSE",
        effective_from=date(2024, 1, 1),
        effective_to=None,
        status=MemberStatus.ACTIVE,
        weight=1.0,
    )

    u1 = UniverseDefinition(
        universe_id="TEST-HASH",
        version="v1",
        name="Test Hash Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=[m1, m2],
    )

    u2 = UniverseDefinition(
        universe_id="TEST-HASH",
        version="v1",
        name="Test Hash Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=[m2, m1],  # Reverse insertion order
    )

    # Hashes must be identical regardless of insertion order
    hash1 = u1.content_hash()
    hash2 = u2.content_hash()
    assert hash1 == hash2
    assert len(hash1) == 64

    # Changing a parameter must alter the hash
    m2_modified = UniverseMember(
        symbol="HPG",
        exchange="HOSE",
        effective_from=date(2024, 1, 1),
        effective_to=None,
        status=MemberStatus.ACTIVE,
        weight=1.5,  # Modified weight
    )
    u3 = UniverseDefinition(
        universe_id="TEST-HASH",
        version="v1",
        name="Test Hash Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=[m1, m2_modified],
    )
    assert u3.content_hash() != hash1


# ---------------------------------------------------------------------------
# 2. Structural Validation & Interval Integrity
# ---------------------------------------------------------------------------

def test_universe_validation_errors():
    """Verify UniverseResolver.validate_definition rejects corrupt or contradictory definitions."""
    # Empty universe_id
    with pytest.raises(ValueError, match="universe_id cannot be empty"):
        UniverseResolver.validate_definition(
            UniverseDefinition(
                universe_id="",
                version="v1",
                name="Bad",
                status=UniverseStatus.CANDIDATE,
                default_mode=UniverseMode.POINT_IN_TIME,
            )
        )

    # Empty version
    with pytest.raises(ValueError, match="version cannot be empty"):
        UniverseResolver.validate_definition(
            UniverseDefinition(
                universe_id="TEST",
                version="  ",
                name="Bad",
                status=UniverseStatus.CANDIDATE,
                default_mode=UniverseMode.POINT_IN_TIME,
            )
        )

    # Member invalid date interval
    with pytest.raises(ValueError, match="invalid interval"):
        m_bad_interval = UniverseMember(
            symbol="VNM",
            exchange="HOSE",
            effective_from=date(2025, 1, 1),
            effective_to=date(2024, 1, 1),  # to < from
            status=MemberStatus.ACTIVE,
        )
        UniverseResolver.validate_definition(
            UniverseDefinition(
                universe_id="TEST",
                version="v1",
                name="Bad",
                status=UniverseStatus.CANDIDATE,
                default_mode=UniverseMode.POINT_IN_TIME,
                members=[m_bad_interval],
            )
        )

    # Overlapping active episodes for same symbol
    with pytest.raises(ValueError, match="Overlapping episodes detected"):
        ep1 = UniverseMember(
            symbol="VNM",
            exchange="HOSE",
            effective_from=date(2024, 1, 1),
            effective_to=date(2024, 6, 30),
            status=MemberStatus.ACTIVE,
        )
        ep2 = UniverseMember(
            symbol="VNM",
            exchange="HOSE",
            effective_from=date(2024, 6, 15),  # Overlaps ep1
            effective_to=date(2024, 12, 31),
            status=MemberStatus.ACTIVE,
        )
        UniverseResolver.validate_definition(
            UniverseDefinition(
                universe_id="TEST",
                version="v1",
                name="Bad",
                status=UniverseStatus.CANDIDATE,
                default_mode=UniverseMode.POINT_IN_TIME,
                members=[ep1, ep2],
            )
        )


# ---------------------------------------------------------------------------
# 3. Point-in-Time Resolution & Interval Enforcement (UNI-PIT-001)
# ---------------------------------------------------------------------------

def test_universe_point_in_time_filtering():
    """Verify POINT_IN_TIME resolution enforces member and universe active dates."""
    registry = get_universe_registry()
    pit_defn = registry.get("TEST-PIT", "fixture_v1")

    # 1. Prior to universe effective_from (2024-01-01) -> 0 active members
    r_pre = UniverseResolver.resolve(pit_defn, as_of=date(2023, 12, 31))
    assert r_pre.is_point_in_time is True
    assert r_pre.active_count == 0
    assert r_pre.active_symbols == []
    assert r_pre.survivor_bias_caveat is None

    # 2. 2024-03-01: AAA and CCC active; BBB not yet listed; DDD suspended; EEE reserve
    r_mid1 = UniverseResolver.resolve(pit_defn, as_of=date(2024, 3, 1))
    assert r_mid1.is_point_in_time is True
    assert r_mid1.active_symbols == ["AAA", "CCC"]
    assert "BBB" in r_mid1.excluded_symbols
    assert "DDD" in r_mid1.excluded_symbols
    assert "EEE" in r_mid1.excluded_symbols

    # 3. 2024-07-01: BBB listed, so AAA, BBB, CCC active
    r_mid2 = UniverseResolver.resolve(pit_defn, as_of=date(2024, 7, 1))
    assert r_mid2.active_symbols == ["AAA", "BBB", "CCC"]

    # 4. 2025-07-01: CCC delisted on 2025-06-30, so only AAA, BBB active
    r_post_delist = UniverseResolver.resolve(pit_defn, as_of=date(2025, 7, 1))
    assert r_post_delist.active_symbols == ["AAA", "BBB"]
    assert "CCC" in r_post_delist.excluded_symbols


# ---------------------------------------------------------------------------
# 4. Retrospective Mode & Survivor Bias Caveat (UNI-PIT-001)
# ---------------------------------------------------------------------------

def test_universe_retrospective_caveat():
    """Verify RETROSPECTIVE_FIXED mode marks is_point_in_time=False and populates warning."""
    registry = get_universe_registry()
    pit_defn = registry.get("TEST-PIT", "fixture_v1")

    r_retro = UniverseResolver.resolve(pit_defn, as_of=date(2024, 3, 1), mode=UniverseMode.RETROSPECTIVE_FIXED)
    assert r_retro.is_point_in_time is False
    assert r_retro.mode == UniverseMode.RETROSPECTIVE_FIXED
    # In retrospective mode, all members designated ACTIVE are target ('AAA', 'BBB', 'CCC')
    assert r_retro.active_symbols == ["AAA", "BBB", "CCC"]
    assert r_retro.survivor_bias_caveat is not None
    assert "RETROSPECTIVE_FIXED MODE" in r_retro.survivor_bias_caveat
    assert "survivorship bias" in r_retro.survivor_bias_caveat


# ---------------------------------------------------------------------------
# 5. Coverage Calculation (UNI-COV-002)
# ---------------------------------------------------------------------------

def test_universe_coverage_calculation():
    """Verify UniverseResolver.check_coverage correctly computes ratios and status."""
    registry = get_universe_registry()
    pit_defn = registry.get("TEST-PIT", "fixture_v1")
    resolution = UniverseResolver.resolve(pit_defn, as_of=date(2024, 7, 1))
    # Target is ['AAA', 'BBB', 'CCC'] (count: 3)

    # 1. Full coverage (100%)
    c1 = UniverseResolver.check_coverage(resolution, {"AAA", "BBB", "CCC", "EXTRA"}, threshold=0.80)
    assert c1.status == CoverageStatus.SATISFIED
    assert c1.target_count == 3
    assert c1.available_count == 3
    assert c1.missing_count == 0
    assert c1.coverage_ratio == 1.0
    assert c1.missing_symbols == []

    # 2. Degraded coverage (2/3 = 66.67% >= 50% but < 80%)
    c2 = UniverseResolver.check_coverage(resolution, {"AAA", "BBB"}, threshold=0.80)
    assert c2.status == CoverageStatus.DEGRADED
    assert c2.target_count == 3
    assert c2.available_count == 2
    assert c2.missing_count == 1
    assert c2.missing_symbols == ["CCC"]
    assert c2.coverage_ratio == 0.6667

    # 3. Failed coverage (1/3 = 33.33% < 50%)
    c3 = UniverseResolver.check_coverage(resolution, {"AAA"}, threshold=0.80)
    assert c3.status == CoverageStatus.FAILED
    assert c3.missing_count == 2
    assert c3.missing_symbols == ["BBB", "CCC"]

    # 4. Invalid threshold
    with pytest.raises(ValueError, match="threshold must be between"):
        UniverseResolver.check_coverage(resolution, {"AAA"}, threshold=1.5)


# ---------------------------------------------------------------------------
# 6. SUMI-420 Candidate Artifact Integrity (UNI-420-003)
# ---------------------------------------------------------------------------

def test_sumi420_candidate_integrity():
    """Verify SUMI420_v1_candidate artifact conforms to schema, candidate rules, and constituent counts."""
    registry = get_universe_registry()
    s420 = registry.get("SUMI-420", "v1_candidate")

    assert s420.universe_id == "SUMI-420"
    assert s420.version == "v1_candidate"
    assert s420.status == UniverseStatus.CANDIDATE
    assert s420.default_mode == UniverseMode.RETROSPECTIVE_FIXED
    assert s420.effective_from == date(2026, 9, 11)

    # Verify total audited members from Doraemon (470)
    assert len(s420.members) == 470

    # Exactly 420 core active candidates and 50 reserve candidates
    active_members = [m for m in s420.members if m.status == MemberStatus.ACTIVE]
    reserve_members = [m for m in s420.members if m.status == MemberStatus.RESERVE]
    assert len(active_members) == 420
    assert len(reserve_members) == 50

    # All symbols must be unique 3-letter uppercase tickers
    symbols = [m.symbol for m in s420.members]
    assert len(symbols) == len(set(symbols))
    for sym in symbols:
        assert len(sym) == 3
        assert sym.isupper()
        assert sym.isalpha() or sym.isalnum()

    # Retrospective resolution on pre-inception date yields 420 active targets with warning caveat
    r_retro = UniverseResolver.resolve(s420, as_of=date(2024, 1, 1), mode=UniverseMode.RETROSPECTIVE_FIXED)
    assert r_retro.active_count == 420
    assert r_retro.is_point_in_time is False
    assert "survivorship bias" in r_retro.survivor_bias_caveat

    # Point-in-time resolution on pre-inception date yields 0 active targets (strictly preventing accidental lookahead backfilling)
    r_pit = UniverseResolver.resolve(s420, as_of=date(2024, 1, 1), mode=UniverseMode.POINT_IN_TIME)
    assert r_pit.active_count == 0
    assert r_pit.is_point_in_time is True
    assert r_pit.survivor_bias_caveat is None


# ---------------------------------------------------------------------------
# 7. Universe API Endpoints Integration
# ---------------------------------------------------------------------------

def test_universe_api_endpoints():
    """Verify HTTP contracts for /api/universe routes."""
    client = TestClient(app)

    # 1. List universes
    res_list = client.get("/api/universe/list")
    assert res_list.status_code == 200
    data_list = res_list.json()
    assert isinstance(data_list, list)
    u_ids = [d["universe_id"] for d in data_list]
    assert "SUMI-420" in u_ids
    assert "TEST-PIT" in u_ids

    # 2. Get definition detail
    res_def = client.get("/api/universe/SUMI-420/definition")
    assert res_def.status_code == 200
    data_def = res_def.json()
    assert data_def["universe_id"] == "SUMI-420"
    assert data_def["status"] == "CANDIDATE"
    assert data_def["total_member_count"] == 470
    assert data_def["active_member_count"] == 420
    assert len(data_def["members"]) == 470
    assert len(data_def["selection_rules"]) > 0

    # 3. Non-existent universe definition
    res_not_found = client.get("/api/universe/UNKNOWN_XYZ/definition")
    assert res_not_found.status_code == 404

    # 4. Resolve universe
    res_resolve = client.get("/api/universe/TEST-PIT/resolve?as_of=2024-03-01")
    assert res_resolve.status_code == 200
    data_resolve = res_resolve.json()
    assert data_resolve["as_of"] == "2024-03-01"
    assert data_resolve["active_symbols"] == ["AAA", "CCC"]
    assert data_resolve["is_point_in_time"] is True

    # 5. Check coverage
    res_cov = client.post(
        "/api/universe/TEST-PIT/coverage?as_of=2024-03-01",
        json={"symbols": ["AAA", "CCC"], "threshold": 0.80},
    )
    assert res_cov.status_code == 200
    data_cov = res_cov.json()
    assert data_cov["status"] == "SATISFIED"
    assert data_cov["coverage_ratio"] == 1.0
    assert data_cov["missing_count"] == 0

    # 6. Degraded coverage
    res_cov_deg = client.post(
        "/api/universe/TEST-PIT/coverage?as_of=2024-03-01",
        json={"symbols": ["AAA"], "threshold": 0.80},
    )
    assert res_cov_deg.status_code == 200
    data_cov_deg = res_cov_deg.json()
    assert data_cov_deg["status"] == "DEGRADED"
    assert data_cov_deg["coverage_ratio"] == 0.5
    assert data_cov_deg["missing_symbols"] == ["CCC"]
