# 06 - Audit and Coverage

The Audit Bridge: trace maps, coverage reports, and querying specs.

## Concepts Introduced

- **Trace maps** — See exactly which IR constructs implement each spec requirement
- **Coverage analysis** — How much of the spec has been compiled and verified
- **Querying** — Pull specific items from a spec (entities, actions, invariants, by name)
- **JSON output** — Machine-readable output for agent consumption

## Try It

```bash
# Full audit trace map — see spec-to-IR mapping
intent audit user_service.intent

# Coverage summary
intent coverage user_service.intent

# Query specific item types
intent query user_service.intent entities
intent query user_service.intent actions
intent query user_service.intent invariants
intent query user_service.intent edge-cases

# Query by name
intent query user_service.intent Login
intent query user_service.intent UniqueEmails

# Query verification obligations
intent query user_service.intent obligations

# JSON output (for agents or scripts)
intent --output json audit user_service.intent
intent --output json coverage user_service.intent
intent --output json query user_service.intent actions
```

## Understanding the Trace Map

The audit trace map shows how every spec element maps to IR:

```
Entity User [L14]
  field:id         -> Struct field (UUID)
  field:email      -> Struct field (Email)
  ...

Action Login [L48]
  param:user           -> Function parameter
  param:password_hash  -> Function parameter
  requires[0]          -> Precondition: user.status == Active
  ensures[0]           -> Postcondition: user.login_count == ...
  property:audit_logged -> Annotation
  ...
```

Every line is a proof that the IR didn't invent behavior — it all traces back to the spec.

## Coverage Report

The coverage summary tells you:
- How many entities, actions, invariants, and edge cases exist
- How many fields, parameters, pre/postconditions, and properties are compiled
- Overall coverage percentage

This is how you verify that nothing in the spec was accidentally dropped during compilation.

## Pre-compiled IR

The file `user_service.ir.json` contains the pre-compiled IR. The audit and coverage commands operate on this compiled output. Regenerate it with:

```bash
intent compile user_service.intent > user_service.ir.json
```
