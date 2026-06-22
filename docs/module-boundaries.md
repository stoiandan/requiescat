# Module Boundaries Know-How

This project should be built around modules that protect clear boundaries. A module should expose a small, useful API in the language of the caller, while keeping its internal mechanics private.

## Core Principle

Callers should depend on what a module does, not how it does it.

For example, application code should ask persistence to load or save a cemetery. It should not need to know about SQLite connections, SQL statements, schema validation, migrations, or transaction details. Those are persistence concerns, so they belong inside the persistence module.

Good boundaries make code easier to read, test, and change. They also make the app feel smaller: each layer gets to think in its own vocabulary.

## What A Good Module API Looks Like

A good module API is:

- Clear: the caller can understand the operation without reading the implementation.
- Narrow: it exposes only what callers actually need.
- Stable: internal representation can change without forcing unrelated code to change.
- Domain-oriented: it speaks in project concepts like `Cemetery`, `Grave`, and `Person`, not low-level machinery.
- Hard to misuse: invalid states and ordering requirements are hidden or made explicit.

Prefer APIs like:

```rust
repository.load()
repository.save(cemetery)
```

over APIs that force callers to coordinate low-level details:

```rust
let connection = open_connection(path);
validate_schema(&connection);
load_graves(&connection);
load_people(&connection);
```

The second shape leaks too much. It makes every caller responsible for remembering persistence rules.

## Information That Should Not Leak

Implementation details should stay behind the module boundary unless callers truly need them.

Examples of details to keep private:

- Database connection objects.
- SQL table and column names.
- Migration steps.
- Transaction ordering.
- Cache invalidation details.
- File naming and directory layout rules.
- Serialization formats.
- UI state used only to render or coordinate one screen.

Leaking details is not only about visibility modifiers. A public function that requires callers to pass implementation-specific values can leak just as much as a public struct field.

## Repository Pattern In This Project

Persistence should follow the repository shape already present in the codebase:

```rust
pub trait CemeteryRepository {
    fn load(&self) -> Result<Cemetery, PersistenceError>;
    fn save(&mut self, cemetery: &Cemetery) -> Result<(), PersistenceError>;
}
```

The app layer can choose a repository and call `load` or `save`. The repository handles the rest:

- Opening read-only or writable connections.
- Configuring database settings.
- Initializing schema for new files.
- Validating existing files.
- Running migrations.
- Translating database rows into domain models.
- Translating domain models back into database rows.
- Performing atomic writes with transactions.

This lets the app layer work with `Cemetery`, not `rusqlite::Connection`.

## Dependency Direction

Keep dependencies flowing from higher-level orchestration to lower-level implementation:

```text
App / UI
  -> domain-facing module API
  -> implementation detail
  -> external library or file system
```

For persistence, that means:

```text
App / UI
  -> CemeteryRepository
  -> SqliteCemeteryRepository
  -> rusqlite::Connection
  -> .sqlite file
```

Avoid reversing this direction. Low-level modules should not reach upward into UI state or application orchestration.

## When Adding New Code

Before adding a public function, type, or module, ask:

- Is this API in the caller's vocabulary?
- Does the caller need this detail, or am I leaking implementation?
- Could I change the internals later without changing callers?
- Does this module own the rule I am adding?
- Is the module making the easy path the correct path?

If the answer is unclear, prefer a smaller API and expand it only when a real caller needs more.

## Practical Rules

- Keep low-level setup close to the low-level resource.
- Convert external data into domain types as soon as practical.
- Convert domain types back to external formats at the boundary.
- Do not make callers perform setup steps in the correct order if the module can do it for them.
- Prefer one clear operation over several public helper calls that must be chained correctly.
- Prefer functional programming and pure functions when it is feasible. See [Functional Programming Know-How](functional-programming.md) for the fuller project guidance.
- Let ease of reading and simplicity prevail when they conflict with a preferred pattern.
- Isolate unavoidable side effects, such as persistence, file system access, logging, clocks, and UI coordination, at module boundaries.
- Use traits when the boundary is useful for testing, alternate implementations, or clearer ownership.
- Avoid adding abstractions just to add layers; add them when they protect a real boundary.

## Review Checklist

When reviewing changes, look for boundary leaks:

- Does UI code know about storage details?
- Does persistence know about UI details?
- Are implementation-specific types appearing in unrelated modules?
- Are callers forced to remember ordering rules?
- Are errors reported at the right abstraction level?
- Can the module be tested through its public API?

The goal is not to hide everything. The goal is to reveal the right things: a clear, easy-to-use API that lets the rest of the code stay focused on its own job.
