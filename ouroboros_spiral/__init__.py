"""Boehm spiral-model cycle controller for Ouroboros.

Ouroboros already owns execution (`ooo auto`, `ooo run`, `ooo evolve`). Its
evolve loop is quality-driven: it iterates until an evaluation gate passes.
The spiral model is risk-driven instead - each cycle exists to retire the
single largest risk, and a commitment review decides whether the next cycle
is worth its cost.

This plugin adds only that missing layer: the risk register, top-risk
selection, and the commitment gate. It never executes work. `open_cycle`
emits a spike brief that the operator (or agent) hands to the runtime.
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Iterable

STATE_PATH = Path(".ouroboros") / "spiral.json"

DECISIONS = ("continue", "pivot", "stop")
#: A risk still steers cycles until it is resolved or explicitly accepted.
OPEN_STATES = ("open", "mitigating")
RISK_STATES = OPEN_STATES + ("resolved", "accepted")


class SpiralError(Exception):
    """Operator-facing failure. The CLI renders it as a JSON error."""


def _now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _state_file(root: Path) -> Path:
    return root / STATE_PATH


def load(root: Path) -> dict[str, Any]:
    path = _state_file(root)
    if not path.exists():
        raise SpiralError(
            "no spiral here - run `ooo spiral start \"<objective>\"` first"
        )
    return json.loads(path.read_text(encoding="utf-8"))


def save(root: Path, state: dict[str, Any]) -> None:
    path = _state_file(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(state, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def exposure(risk: dict[str, Any]) -> float:
    """Boehm's risk exposure: probability of loss x size of loss."""
    return round(risk["probability"] * risk["impact"], 4)


def ranked_open_risks(state: dict[str, Any]) -> list[dict[str, Any]]:
    """Open risks, highest exposure first. Ties break by id for determinism."""
    return sorted(
        (r for r in state["risks"] if r["status"] in OPEN_STATES),
        key=lambda r: (-exposure(r), r["id"]),
    )


def _pending_cycle(state: dict[str, Any]) -> dict[str, Any] | None:
    for entry in reversed(state["cycles"]):
        if entry["decision"] is None:
            return entry
    return None


def _find_risk(state: dict[str, Any], risk_id: str) -> dict[str, Any]:
    for risk in state["risks"]:
        if risk["id"] == risk_id:
            return risk
    raise SpiralError(f"unknown risk {risk_id!r}")


def _check_unit(value: float, label: str) -> None:
    if not 0.0 <= value <= 1.0:
        raise SpiralError(f"{label} must be within 0.0..1.0, got {value}")


def start(
    root: Path,
    objective: str,
    *,
    constraints: Iterable[str] = (),
    alternatives: Iterable[str] = (),
) -> dict[str, Any]:
    """Quadrant 1 of cycle 0: fix objectives, alternatives, constraints."""
    if not objective.strip():
        raise SpiralError("objective must not be empty")
    path = _state_file(root)
    if path.exists():
        raise SpiralError(f"spiral already exists at {path} - see `ooo spiral status`")
    state = {
        "objective": objective,
        "created_at": _now(),
        "status": "active",
        "cycle": 0,
        "cumulative_cost": 0.0,
        "constraints": list(constraints),
        "alternatives": list(alternatives),
        "risks": [],
        "cycles": [],
    }
    save(root, state)
    return {
        "objective": objective,
        "status": "active",
        "state_file": str(path),
        "next": "ooo spiral risk add \"<risk>\" --probability 0.7 --impact 0.9",
    }


def add_risk(
    root: Path,
    description: str,
    probability: float,
    impact: float,
    *,
    mitigation: str = "",
) -> dict[str, Any]:
    """Quadrant 2: register a risk so it can compete for the next cycle."""
    if not description.strip():
        raise SpiralError("risk description must not be empty")
    _check_unit(probability, "probability")
    _check_unit(impact, "impact")
    state = load(root)
    risk = {
        # Risks are never deleted, so a monotonic counter cannot collide.
        "id": f"R{len(state['risks']) + 1}",
        "description": description,
        "probability": probability,
        "impact": impact,
        "status": "open",
        "mitigation": mitigation,
        "evidence": "",
        "cycle_opened": state["cycle"],
        "cycle_closed": None,
    }
    state["risks"].append(risk)
    save(root, state)
    return {"risk": risk, "exposure": exposure(risk)}


def close_risk(
    root: Path, risk_id: str, *, status: str = "resolved", evidence: str = ""
) -> dict[str, Any]:
    if status not in ("resolved", "accepted"):
        raise SpiralError("status must be 'resolved' or 'accepted'")
    state = load(root)
    risk = _find_risk(state, risk_id)
    risk["status"] = status
    risk["evidence"] = evidence
    risk["cycle_closed"] = state["cycle"]
    save(root, state)
    return {"risk": risk}


def list_risks(root: Path) -> dict[str, Any]:
    state = load(root)
    return {
        "open": [dict(r, exposure=exposure(r)) for r in ranked_open_risks(state)],
        "closed": [
            dict(r, exposure=exposure(r))
            for r in state["risks"]
            if r["status"] not in OPEN_STATES
        ],
    }


def _brief(state: dict[str, Any], risk: dict[str, Any], cycle_n: int) -> dict[str, Any]:
    return {
        "cycle": cycle_n,
        "objective": state["objective"],
        "de_risk": risk["description"],
        "risk_id": risk["id"],
        "exposure": exposure(risk),
        "planned_mitigation": risk["mitigation"],
        "constraints": state["constraints"],
        "instruction": (
            f"Cycle {cycle_n} exists to retire exactly one risk: "
            f"{risk['description']} (exposure {exposure(risk):.2f}). Build the "
            "smallest prototype or spike that produces evidence for or against "
            "it, then verify. Do not broaden scope to unrelated work - other "
            "risks get their own cycles."
        ),
    }


def open_cycle(
    root: Path, *, objectives: Iterable[str] = ()
) -> dict[str, Any]:
    """Quadrants 1-3: pick the top risk and emit the spike brief for it."""
    state = load(root)
    if state["status"] != "active":
        raise SpiralError(f"spiral is {state['status']} - no further cycles")
    pending = _pending_cycle(state)
    if pending is not None:
        raise SpiralError(
            f"cycle {pending['n']} is still open - close it with "
            "`ooo spiral commit --decision <continue|pivot|stop>`"
        )
    ranked = ranked_open_risks(state)
    if not ranked:
        raise SpiralError(
            "no open risks - a spiral cycle must be driven by one. Add a risk "
            "with `ooo spiral risk add`, or close out with `ooo spiral stop`."
        )
    top = ranked[0]
    state["cycle"] += 1
    entry = {
        "n": state["cycle"],
        "opened_at": _now(),
        "objectives": list(objectives) or [state["objective"]],
        "top_risk": top["id"],
        "top_risk_exposure": exposure(top),
        "decision": None,
        "cost": 0.0,
        "outcome": "",
    }
    state["cycles"].append(entry)
    if top["status"] == "open":
        top["status"] = "mitigating"
    save(root, state)
    return {
        "cycle": entry["n"],
        "brief": _brief(state, top, entry["n"]),
        "next": [
            "hand the brief to the runtime, e.g. `ooo auto \"<instruction>\"`",
            "then `ooo spiral commit --decision continue --cost <n> "
            f"--resolve {top['id']}`",
        ],
    }


def commit(
    root: Path,
    decision: str,
    *,
    cost: float = 0.0,
    outcome: str = "",
    resolve: Iterable[str] = (),
    evidence: str = "",
) -> dict[str, Any]:
    """Quadrant 4: the commitment review that closes a cycle."""
    if decision not in DECISIONS:
        raise SpiralError(f"decision must be one of {list(DECISIONS)}")
    if cost < 0:
        raise SpiralError(f"cost must not be negative, got {cost}")
    state = load(root)
    entry = _pending_cycle(state)
    if entry is None:
        raise SpiralError("no open cycle - start one with `ooo spiral cycle`")
    resolved = []
    for risk_id in resolve:
        risk = _find_risk(state, risk_id)
        risk["status"] = "resolved"
        risk["evidence"] = evidence
        risk["cycle_closed"] = entry["n"]
        resolved.append(risk_id)
    entry["decision"] = decision
    entry["cost"] = cost
    entry["outcome"] = outcome
    entry["resolved_risks"] = resolved
    entry["closed_at"] = _now()
    # The spiral's radius: cost accumulated across every cycle so far.
    state["cumulative_cost"] = round(state["cumulative_cost"] + cost, 4)
    if decision == "stop":
        state["status"] = "stopped"
    save(root, state)
    return {
        "cycle": entry["n"],
        "decision": decision,
        "resolved_risks": resolved,
        "cumulative_cost": state["cumulative_cost"],
        "spiral_status": state["status"],
        "remaining_exposure": _total_exposure(state),
    }


def stop(root: Path, *, reason: str = "") -> dict[str, Any]:
    """Abandon the spiral without a cycle in flight."""
    state = load(root)
    state["status"] = "stopped"
    state["stopped_reason"] = reason
    save(root, state)
    return {"spiral_status": "stopped", "reason": reason}


def _total_exposure(state: dict[str, Any]) -> float:
    return round(sum(exposure(r) for r in ranked_open_risks(state)), 4)


def status(root: Path) -> dict[str, Any]:
    state = load(root)
    ranked = ranked_open_risks(state)
    pending = _pending_cycle(state)
    return {
        "objective": state["objective"],
        "spiral_status": state["status"],
        "cycle": state["cycle"],
        # Radius grows with cost; convergence shows as falling exposure.
        "radius": {
            "cycles_completed": sum(
                1 for c in state["cycles"] if c["decision"] is not None
            ),
            "cumulative_cost": state["cumulative_cost"],
        },
        "remaining_exposure": _total_exposure(state),
        "open_risks": [
            {
                "id": r["id"],
                "description": r["description"],
                "exposure": exposure(r),
                "status": r["status"],
            }
            for r in ranked
        ],
        "pending_cycle": pending["n"] if pending else None,
        "history": [
            {
                "n": c["n"],
                "top_risk": c["top_risk"],
                "decision": c["decision"],
                "cost": c["cost"],
            }
            for c in state["cycles"]
        ],
    }
