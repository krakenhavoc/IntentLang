# 04 - Invariants and Edge Cases

System-wide constraints with `forall`/`exists` quantifiers and explicit edge case handling.

## Concepts Introduced

- `invariant` — Constraints that must hold at all times across the entire system
- `forall x: Type => predicate` — Universal quantification ("for every X, this must be true")
- `exists x: Type => predicate` — Existential quantification ("there must be some X where this is true")
- `!(exists ...)` — Negated existence ("no such X should exist")
- `edge_cases` with `when ... => reject(...)` — Explicit boundary condition handling

## Try It

```bash
# Check the spec
intent check voting.intent

# See the full audit trace map
intent audit voting.intent

# Check coverage
intent coverage voting.intent

# Verify IR
intent verify voting.intent
```

## Key Insight

Invariants are the backbone of formal specification. They express properties that must **always** hold, regardless of which actions run in which order:

```intent
invariant OneVotePerVoter {
  forall b1: Ballot =>
    forall b2: Ballot =>
      b1.voter == b2.voter &&
      b1.election == b2.election =>
        b1 == b2
}
```

This says: "If two ballots share the same voter and election, they must be the same ballot." This is a uniqueness constraint — no double voting, period. The verifier checks that every action preserves this invariant.

Edge cases make boundary handling explicit rather than hiding it in implementation details. A product manager can read `when voter.status == Suspended => reject(...)` and confirm the business rule.

## Compiled IR

The file `voting.ir.json` contains the pre-compiled Agent IR. Regenerate it with:

```bash
intent compile voting.intent > voting.ir.json
```

The IR includes invariants and edge guards alongside the structs and functions — these are the verification obligations the system checks.
