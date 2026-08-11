"""Self-check for the spiral controller. Run: python3 test_spiral.py"""

from __future__ import annotations

import tempfile
from pathlib import Path

import ouroboros_spiral as sp


def _raises(fn, needle: str) -> None:
    try:
        fn()
    except sp.SpiralError as exc:
        assert needle in str(exc), f"expected {needle!r} in {exc!r}"
        return
    raise AssertionError(f"expected SpiralError containing {needle!r}")


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)

        _raises(lambda: sp.status(root), "no spiral here")
        sp.start(root, "ship realtime editing", constraints=["one Postgres box"])
        _raises(lambda: sp.start(root, "again"), "already exists")

        _raises(lambda: sp.add_risk(root, "bad", 1.5, 0.5), "probability")
        _raises(lambda: sp.add_risk(root, "", 0.5, 0.5), "must not be empty")

        # A cycle needs a risk to drive it.
        _raises(lambda: sp.open_cycle(root), "no open risks")

        sp.add_risk(root, "low risk", 0.2, 0.3)          # R1, exposure 0.06
        sp.add_risk(root, "CRDT memory blowup", 0.6, 0.9)  # R2, exposure 0.54
        sp.add_risk(root, "auth mismatch", 0.4, 0.7)       # R3, exposure 0.28

        # Highest exposure wins, regardless of insertion order.
        assert [r["id"] for r in sp.ranked_open_risks(sp.load(root))] == [
            "R2",
            "R3",
            "R1",
        ]

        opened = sp.open_cycle(root)
        assert opened["cycle"] == 1
        assert opened["brief"]["risk_id"] == "R2"
        assert opened["brief"]["exposure"] == 0.54
        # Selecting a risk marks it as being worked, not resolved.
        assert sp.load(root)["risks"][1]["status"] == "mitigating"

        # One cycle at a time: the commitment gate is not skippable.
        _raises(lambda: sp.open_cycle(root), "still open")

        _raises(lambda: sp.commit(root, "maybe"), "decision must be one of")
        _raises(lambda: sp.commit(root, "continue", cost=-1), "must not be negative")
        _raises(
            lambda: sp.commit(root, "continue", resolve=["R99"]), "unknown risk"
        )

        before = sp.status(root)["remaining_exposure"]
        done = sp.commit(root, "continue", cost=1.5, resolve=["R2"], evidence="380MB")
        assert done["resolved_risks"] == ["R2"]
        assert done["cumulative_cost"] == 1.5
        # Retiring a risk must shrink remaining exposure.
        assert done["remaining_exposure"] == round(before - 0.54, 4) < before

        # Next cycle picks the new leader and cost accumulates as radius.
        assert sp.open_cycle(root)["brief"]["risk_id"] == "R3"
        sp.commit(root, "continue", cost=2.0)
        st = sp.status(root)
        assert st["radius"] == {"cycles_completed": 2, "cumulative_cost": 3.5}, st
        # Inconclusive spike: R3 was not resolved, so it stays in the running.
        assert st["open_risks"][0]["id"] == "R3"

        # An accepted risk stops steering cycles but stays in the record.
        sp.close_risk(root, "R3", status="accepted", evidence="ship with fallback")
        assert [r["id"] for r in sp.ranked_open_risks(sp.load(root))] == ["R1"]

        # `stop` ends the spiral; no further cycles.
        sp.open_cycle(root)
        sp.commit(root, "stop", cost=0.5, outcome="not worth it")
        assert sp.status(root)["spiral_status"] == "stopped"
        _raises(lambda: sp.open_cycle(root), "no further cycles")

    print("ok")


if __name__ == "__main__":
    main()
