# Modules

Every `.intent` file begins with a module declaration:

```intent
module TransferFunds
```

The module name must start with an uppercase letter and use PascalCase. It serves as a namespace for all definitions in the file.

## Documentation blocks

Documentation blocks use `---` and can appear after the module declaration or before any entity, action, or invariant:

```intent
module TransferFunds

--- A fund transfer system between accounts within the same currency.
--- Supports basic account-to-account transfers with balance validation,
--- idempotency guarantees, and manager approval for large amounts.
```

Documentation blocks are preserved in rendered output and audit traces, making them useful for explaining design intent to both humans and tooling.
