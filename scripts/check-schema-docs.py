#!/usr/bin/env python3
"""Generate and verify MCP schema/action documentation drift."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCHEMAS_RS = ROOT / "src/mcp/schemas.rs"
ACTION_RS = ROOT / "src/actions.rs"
TOOLS_RS = ROOT / "src/mcp/tools.rs"
PROMPTS_RS = ROOT / "src/mcp/prompts.rs"
RMCP_SERVER_RS = ROOT / "src/mcp/rmcp_server.rs"
README = ROOT / "README.md"
SKILL = ROOT / "plugins/rarcane/skills/rarcane/SKILL.md"
DOC = ROOT / "docs/MCP_SCHEMA.md"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def extract_actions() -> list[str]:
    return list(extract_scope_for_actions())


def extract_scope_for_actions() -> dict[str, str]:
    text = read(ACTION_RS)
    registry = text.split("pub const ACTION_SPECS", 1)[1].split("\n];", 1)[0]
    raw_scopes: dict[str, set[str]] = {}
    entries = re.findall(r"ActionSpec\s*\{(.*?)\}", registry, re.S)
    for entry in entries:
        name_match = re.search(r'action:\s*"([^"]+)"', entry)
        scope_match = re.search(r"required_scope:\s*([^,\n]+)", entry)
        if not name_match or not scope_match:
            continue
        name = name_match.group(1)
        scope_expr = scope_match.group(1).strip()
        if scope_expr == "None":
            raw_scopes.setdefault(name, set()).add("public")
        elif scope_expr == "Some(READ_SCOPE)":
            raw_scopes.setdefault(name, set()).add("read")
        elif scope_expr == "Some(WRITE_SCOPE)":
            raw_scopes.setdefault(name, set()).add("write")
        else:
            raw_scopes.setdefault(name, set()).add("deny")
    for action, scope in re.findall(
        r'spec!\(\s*"([^"]+)"\s*,\s*"[^"]+"\s*,.*?\b(READ_SCOPE|WRITE_SCOPE)\b',
        registry,
        re.S,
    ):
        raw_scopes.setdefault(action, set()).add("read" if scope == "READ_SCOPE" else "write")

    scopes: dict[str, str] = {}
    for action, values in raw_scopes.items():
        if values == {"public"}:
            scopes[action] = "public"
        elif values == {"read"}:
            scopes[action] = "`rarcane:read`"
        elif values == {"write"}:
            scopes[action] = "`rarcane:write`"
        elif values <= {"read", "write"}:
            scopes[action] = "`rarcane:read` / `rarcane:write`"
        else:
            scopes[action] = "`rarcane:__deny__`"
    return scopes


def action_description(action: str) -> str:
    descriptions = {
        "status": "Return local server status without contacting Arcane.",
        "help": "Return the in-tool action reference. Public; no scope required.",
        "elicit_name": "Ask the MCP client for a name and return a greeting.",
        "scaffold_intent": "Collect scaffold requirements and return a side-effect-free handoff contract.",
        "environment": "List, inspect, test, create, update, and delete Arcane environments.",
        "project": "List, inspect, deploy, build, and manage Arcane projects.",
        "container": "List, inspect, start, stop, restart, update, and remove containers.",
        "image": "List, inspect, pull, prune, scan, and remove images.",
        "network": "List, inspect, create, prune, and remove networks.",
        "volume": "List, inspect, browse, back up, restore, create, prune, and remove volumes.",
        "system": "Retrieve system information and run supported system operations.",
        "image-update": "Check image update status and summaries.",
        "vulnerability": "Inspect, ignore, and unignore vulnerability findings.",
        "registry": "List, inspect, test, create, update, and delete registries.",
        "gitops": "List, inspect, browse, sync, create, update, and delete GitOps entries.",
    }
    return descriptions.get(action, "TEMPLATE: document this action.")


def render() -> str:
    actions = extract_actions()
    scopes = extract_scope_for_actions()
    lines = [
        "---",
        'title: "MCP Schema Contract"',
        'created: "2026-07-30"',
        'updated: "2026-07-30"',
        "---",
        "",
        "# MCP Schema Contract",
        "",
        "Generated from `src/actions.rs` and checked against the schema, README, skill docs, help text, and scope routing.",
        "",
        "Run:",
        "",
        "```bash",
        "python3 scripts/check-schema-docs.py --write",
        "python3 scripts/check-schema-docs.py --check",
        "```",
        "",
        "## Tool",
        "",
        "| Field | Value |",
        "|---|---|",
        "| Tool name | `rarcane` |",
        "| Schema resource | `rarcane://schema/mcp-tool` |",
        "| Dispatch parameter | `action` |",
        "",
        "## Actions",
        "",
        "| Action | Scope | Description |",
        "|---|---|---|",
    ]
    for action in actions:
        scope = scopes[action]
        lines.append(f"| `{action}` | {scope} | {action_description(action)} |")
    lines.extend(
        [
            "",
            "## Drift Rules",
            "",
            "- `ACTION_SPECS` in `src/actions.rs` is the canonical action and scope list.",
            "- `src/mcp/schemas.rs` must derive its enum from `ACTION_SPECS`.",
            "- The MCP tool schema must reject unknown top-level parameters and encode action-specific requirements that fit the single-tool dispatch model.",
            "- `help` is intentionally public and must have no required scope.",
            "- `src/mcp/tools.rs`, `README.md`, and `plugins/rarcane/skills/rarcane/SKILL.md` must mention every action.",
            "- `src/mcp/rmcp_server.rs` owns stable resources and must keep `rarcane://schema/mcp-tool` wired to `tool_definitions()`.",
            "- `src/mcp/prompts.rs` owns stable prompts and must keep `quick_start` covered by prompt tests.",
            "",
            "## Resources",
            "",
            "| URI | Source | Contract |",
            "|---|---|---|",
            "| `rarcane://schema/mcp-tool` | `src/mcp/rmcp_server.rs` | Returns `tool_definitions()` as `application/json`. |",
            "",
            "## Prompts",
            "",
            "| Prompt | Source | Contract |",
            "|---|---|---|",
            "| `quick_start` | `src/mcp/prompts.rs` | Guides a client to call `status` and public `help`. |",
            "",
            "## Input Validation",
            "",
            "- `action` is always required.",
            "- `elicit_name` and `scaffold_intent` collect their extra fields through MCP elicitation, not direct tool-call arguments.",
            "- Unknown top-level parameters are rejected by the schema.",
            "",
        ]
    )
    return "\n".join(lines)


def check_mentions(actions: list[str]) -> list[str]:
    failures: list[str] = []
    surfaces = {
        "README.md": read(README),
        "plugins/rarcane/skills/rarcane/SKILL.md": read(SKILL),
        "src/actions.rs registry/help": read(ACTION_RS),
    }
    for label, text in surfaces.items():
        for action in actions:
            if action not in text:
                failures.append(f"{label} does not mention action `{action}`")
    return failures


def check_scope(actions: list[str]) -> list[str]:
    failures: list[str] = []
    scopes = extract_scope_for_actions()
    if set(scopes) != set(actions):
        failures.append("ACTION_SPECS action names and scope entries are out of sync")
    if scopes.get("help") != "public":
        failures.append("help must be public")
    for action in set(actions) - {"help"}:
        if scopes.get(action) == "public":
            failures.append(f"action `{action}` must declare a required scope")
    schema_text = read(SCHEMAS_RS)
    if "action_names()" not in schema_text:
        failures.append("src/mcp/schemas.rs must derive action enum from action_names()")
    if '"additionalProperties": false' not in schema_text:
        failures.append("src/mcp/schemas.rs must reject unknown top-level properties")
    rmcp_server_text = read(RMCP_SERVER_RS)
    if "rarcane://schema/mcp-tool" not in rmcp_server_text or "tool_definitions()" not in rmcp_server_text:
        failures.append("src/mcp/rmcp_server.rs must expose the schema resource from tool_definitions()")
    prompts_text = read(PROMPTS_RS)
    if "quick_start" not in prompts_text:
        failures.append("src/mcp/prompts.rs must expose quick_start prompt")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="Rewrite docs/MCP_SCHEMA.md.")
    parser.add_argument("--check", action="store_true", help="Fail if docs or action surfaces drift.")
    args = parser.parse_args()
    if not args.write and not args.check:
        args.check = True

    rendered = render()
    if args.write:
        DOC.write_text(rendered, encoding="utf-8")
        print(f"wrote {DOC.relative_to(ROOT)}")

    failures: list[str] = []
    if args.check:
        if not DOC.exists():
            failures.append("docs/MCP_SCHEMA.md is missing; run --write")
        elif read(DOC) != rendered:
            failures.append("docs/MCP_SCHEMA.md is stale; run --write")
        actions = extract_actions()
        failures.extend(check_mentions(actions))
        failures.extend(check_scope(actions))

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    if args.check:
        print("schema docs are current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
