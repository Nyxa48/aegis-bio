from pathlib import Path

from fastapi.testclient import TestClient

from hub.app import create_app


def _make_client(tmp_path: Path) -> TestClient:
    app = create_app(db_path=tmp_path / "test_hub.db")
    return TestClient(app)


def test_crew_overview_shows_live_heart_rate(tmp_path: Path):
    with _make_client(tmp_path) as client:
        res = client.get("/api/v1/crew")
        assert res.status_code == 200
        crew = res.json()["crew"]
        assert len(crew) == 4
        assert all("heart_rate" in c for c in crew)
        assert crew[0]["subject_id"] == "C001"


def test_profile_preserves_null_measurement_and_sorts_by_event(tmp_path: Path):
    with _make_client(tmp_path) as client:
        res = client.get("/api/v1/crew/C003")
        assert res.status_code == 200
        data = res.json()
        assert data["profile"]["subject_id"] == "C003"
        missing = [m for m in data["measurements"] if m["quality_flag"] == "missing"]
        assert len(missing) == 1
        assert missing[0]["value"] is None


def test_meal_logging_is_idempotent(tmp_path: Path):
    with _make_client(tmp_path) as client:
        payload = {
            "event_id": "meal-evt-001",
            "subject_id": "C001",
            "food_id": "ISS-01",
            "consumed_ratio": 0.5,
        }
        first = client.post("/api/v1/meals", json=payload)
        assert first.status_code == 200
        assert first.json()["status"] == "logged"
        assert first.json()["kcal_added"] == 170.0

        second = client.post("/api/v1/meals", json=payload)
        assert second.status_code == 200
        assert second.json()["status"] == "duplicate_ignored"


def test_telemetry_sync_updates_heart_rate_idempotently(tmp_path: Path):
    with _make_client(tmp_path) as client:
        packets = [
            {
                "event_id": "tel-001",
                "subject_id": "C002",
                "heart_rate": 112.0,
                "oxygen_level": 97.5,
                "suit_pressure": 0.99,
                "temperature": 22.0,
                "mission_phase": "ORBIT_IVA",
            }
        ]
        res1 = client.post("/api/v1/telemetry/sync", json=packets)
        assert res1.status_code == 200
        assert res1.json()["results"][0]["status"] == "accepted"

        res2 = client.post("/api/v1/telemetry/sync", json=packets)
        assert res2.status_code == 200
        assert res2.json()["results"][0]["status"] == "duplicate_ignored"

        crew = client.get("/api/v1/crew").json()["crew"]
        c002 = next(c for c in crew if c["subject_id"] == "C002")
        assert c002["heart_rate"] == 112.0


def test_hub_only_radiation_endpoint(tmp_path: Path):
    with _make_client(tmp_path) as client:
        res = client.get("/api/v1/hub-only/radiation")
        assert res.status_code == 200
        body = res.json()
        assert body["scope"] == "HUB_ONLY_CREW_RESTRICTED"
        assert len(body["readings"]) > 0
        assert body["readings"][0]["instrument_id"] == "Liulin-5-D3"
