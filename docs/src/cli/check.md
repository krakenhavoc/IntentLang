# Checking Specs

```bash
intent check <file>
```

The `check` command parses a `.intent` file and runs six semantic analysis passes:

1. **Collect definitions** — gather all entity, action, and invariant names; detect duplicates
2. **Resolve type references** — verify that all types are either built-in or defined as entities
3. **Validate quantifier bindings** — ensure `forall`/`exists` bind to valid entity or action types
4. **Validate edge case actions** — check that edge case handlers reference valid operations
5. **Validate field access** — verify dotted field access on entity-typed parameters
6. **Constraint validation** — check `old()` placement and detect tautological comparisons

## Success output

```
$ intent check examples/transfer.intent
OK: TransferFunds — 7 top-level item(s), no issues found
```

## Error output

Errors include source spans, labels, and actionable help via [miette](https://crates.io/crates/miette):

```
intent::check::undefined_type

  × undefined type `Customer`
   ╭─[5:13]
 4 │       id: UUID
 5 │ ╭─▶   customer: Customer
 6 │ ├─▶   items: List<LineItem>
   · ╰──── used here
 7 │     }
   ╰────
  help: define an entity named `Customer`, or use a built-in type
```

```
intent::check::old_in_requires

  × `old()` cannot be used in a `requires` block
    ╭─[13:21]
 12 │       requires {
 13 │ ╭─▶     from.balance == old(from.balance)
 14 │ ├─▶   }
    · ╰──── used here
    ╰────
  help: `old()` references pre-state values and is only meaningful in `ensures` blocks
```
