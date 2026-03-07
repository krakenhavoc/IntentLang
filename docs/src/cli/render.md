# Rendering

## Markdown

```bash
intent render <file>
```

Produces a Markdown document with:
- Module name and documentation
- Entity field tables
- Action signatures with pre/postconditions
- Invariant descriptions
- Edge case rules

Suitable for sharing with non-technical stakeholders or embedding in project documentation.

## HTML

```bash
intent render-html <file> > output.html
```

Produces a self-contained HTML document with color-coded sections. No external CSS or JavaScript dependencies — the output is a single file you can open in any browser.

```bash
intent render-html examples/transfer.intent > transfer.html
open transfer.html
```
