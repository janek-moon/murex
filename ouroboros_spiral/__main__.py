"""Command entrypoint for the spiral plugin.

Ouroboros invokes plugins as `<entrypoint.command> <command> [args...]` and
reads a JSON document from stdout, so every path here prints JSON and nothing
else. Failures print `{"error": ...}` and exit 1.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from . import (
    SpiralError,
    add_risk,
    close_risk,
    commit,
    list_risks,
    open_cycle,
    start,
    status,
    stop,
)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="ooo spiral")
    parser.add_argument(
        "--root", type=Path, default=Path("."), help="Repository root."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_start = sub.add_parser("start", help="Open a spiral for an objective.")
    p_start.add_argument("objective")
    p_start.add_argument("--constraint", action="append", default=[])
    p_start.add_argument("--alternative", action="append", default=[])

    p_risk = sub.add_parser("risk", help="Manage the risk register.")
    risk_sub = p_risk.add_subparsers(dest="action", required=True)
    p_add = risk_sub.add_parser("add")
    p_add.add_argument("description")
    p_add.add_argument("--probability", type=float, required=True)
    p_add.add_argument("--impact", type=float, required=True)
    p_add.add_argument("--mitigation", default="")
    risk_sub.add_parser("list")
    p_close = risk_sub.add_parser("close")
    p_close.add_argument("risk_id")
    p_close.add_argument("--status", default="resolved", choices=["resolved", "accepted"])
    p_close.add_argument("--evidence", default="")

    p_cycle = sub.add_parser("cycle", help="Open the next risk-driven cycle.")
    p_cycle.add_argument("--objective", action="append", default=[])

    p_commit = sub.add_parser("commit", help="Commitment review; closes a cycle.")
    p_commit.add_argument(
        "--decision", required=True, choices=["continue", "pivot", "stop"]
    )
    p_commit.add_argument("--cost", type=float, default=0.0)
    p_commit.add_argument("--outcome", default="")
    p_commit.add_argument("--resolve", action="append", default=[])
    p_commit.add_argument("--evidence", default="")

    p_stop = sub.add_parser("stop", help="Abandon the spiral.")
    p_stop.add_argument("--reason", default="")

    sub.add_parser("status", help="Radius, remaining exposure, history.")
    return parser


def _dispatch(args: argparse.Namespace) -> dict:
    root = args.root
    if args.command == "start":
        return start(
            root,
            args.objective,
            constraints=args.constraint,
            alternatives=args.alternative,
        )
    if args.command == "risk":
        if args.action == "add":
            return add_risk(
                root,
                args.description,
                args.probability,
                args.impact,
                mitigation=args.mitigation,
            )
        if args.action == "list":
            return list_risks(root)
        return close_risk(
            root, args.risk_id, status=args.status, evidence=args.evidence
        )
    if args.command == "cycle":
        return open_cycle(root, objectives=args.objective)
    if args.command == "commit":
        return commit(
            root,
            args.decision,
            cost=args.cost,
            outcome=args.outcome,
            resolve=args.resolve,
            evidence=args.evidence,
        )
    if args.command == "stop":
        return stop(root, reason=args.reason)
    return status(root)


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        result = _dispatch(args)
    except SpiralError as exc:
        sys.stdout.write(json.dumps({"error": str(exc)}, ensure_ascii=False) + "\n")
        return 1
    sys.stdout.write(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
