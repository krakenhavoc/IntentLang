# 07 - Multi-Agent Collaboration

Lock/unlock workflow for agents working on specs in parallel.

## Concepts Introduced

- **Lock** — An agent claims a spec item so no other agent modifies it
- **Unlock** — Release a claimed item when done
- **Status** — View which items are claimed and by whom
- **Diff** — Compare two versions of a spec to see what changed

## Try It

```bash
# First, verify the spec
intent verify deployment.intent

# Agent "alpha" claims the Deploy action
intent lock deployment.intent Deploy --agent alpha

# Agent "beta" claims the Rollback action
intent lock deployment.intent Rollback --agent beta

# See who owns what
intent status deployment.intent

# Release claims when done
intent unlock deployment.intent Deploy --agent alpha
intent unlock deployment.intent Rollback --agent beta

# Compare two versions of a spec
# (make a copy, modify it, then diff)
cp deployment.intent deployment_v2.intent
# ... edit deployment_v2.intent ...
intent diff deployment.intent deployment_v2.intent
```

## Multi-Agent Workflow

In a multi-agent scenario, the workflow looks like:

1. **Agent reads the spec** via `intent query` (JSON output)
2. **Agent claims items** via `intent lock` before generating IR
3. **Agent generates implementation** for its claimed items
4. **Agent verifies** its changes don't break invariants
5. **Agent releases claims** via `intent unlock`
6. **Human reviews** via `intent audit` and `intent coverage`

The lock system prevents two agents from modifying the same action or entity simultaneously. The status command gives a dashboard of current ownership.

## JSON API for Agents

All commands support `--output json` for machine consumption:

```bash
intent --output json query deployment.intent actions
intent --output json verify deployment.intent
intent --output json lock deployment.intent Deploy --agent agent-1
intent --output json status deployment.intent
```

## Pre-compiled IR

The file `deployment.ir.json` contains the pre-compiled IR. Regenerate it with:

```bash
intent compile deployment.intent > deployment.ir.json
```
