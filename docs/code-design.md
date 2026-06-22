# Code Design Know-How

This document collects the practical design rules that should guide ordinary changes in Requiescat. It sits alongside [Module Boundaries Know-How](module-boundaries.md) and [Functional Programming Know-How](functional-programming.md).

The short version: make invalid states hard to represent, keep side effects at clear boundaries, and let readability win.

## Design Priorities

When tradeoffs appear, prefer this order:

- Correct domain behavior.
- Readable, direct code.
- Clear module ownership.
- Focused tests around the behavior being changed.
- Functional style and pure helpers where they make the code easier to trust.
- Abstractions only when they remove real complexity.

Patterns are useful only when they serve those priorities.

## Domain Data

The domain layer should use explicit types for project concepts. Prefer a type like `PersonDate`, `GraveGps`, `GraveColor`, `GraveId`, or `PersonId` over passing raw strings or integers through the app.

Domain types should usually own their own validation and formatting rules:

- `PersonDate` parses and formats dates.
- `GraveGps` parses and formats DMS coordinates.
- `GraveColor` converts to UI and persistence representations.
- ID types keep unrelated IDs from being mixed accidentally.

Raw strings are fine at boundaries, especially while the user is typing. Once data becomes part of the saved domain model, convert it into domain values.

## Persistence

Persistence is a boundary. Its job is to translate between SQLite rows and domain values, not to leak SQL details into the rest of the app.

Keep persistence code shaped like this:

```text
open/configure database
validate current schema
read rows
convert rows into domain values
return domain model
```

and:

```text
accept domain model
convert domain values into rows
write rows transactionally
```

Persistence should reject invalid stored data. If a row contains a bad date, bad GPS coordinate, or invalid required value, loading should fail with a clear persistence error rather than silently producing a partial or surprising domain model.

Assume the current schema unless compatibility is an explicit product decision. Do not add migrations, fallback queries, column repair helpers, or old-format behavior "just in case."

Implementation details such as row DTOs, SQL column names, transactions, and schema validation helpers should stay private to the persistence module.

## UI Draft State

UI state may be messier than domain state. That is normal.

For text inputs, it is often useful to keep draft strings even when they are temporarily invalid. For example, while typing a date or GPS coordinate, the user may pass through invalid intermediate states. The UI should preserve that draft text without committing it to the domain model.

Use this shape:

```text
store draft text
try to parse or validate it
if valid, update the domain model
if invalid, keep the draft only
```

Do not replace valid domain values with invalid draft values. Do not make domain types accept invalid data just to make UI input easier.

When a draft represents a complete form, prefer one conversion method that turns the draft into a domain-ready value or returns `None`. Avoid validating in one function and then reparsing the same data later.

## Mutation

Mutation is acceptable when it is the clearest shape for the owner of the state. Aggregates such as `Cemetery`, `CemeteryMap`, `PersonDirectory`, UI components, and repositories can mutate their owned state when that keeps call sites simple.

For value objects and domain transformations, prefer returned values when that makes the rule clearer:

```rust
let moved = grave.translated(delta);
let renamed = person.with_first_name(first_name);
```

Avoid keeping both mutating and value-returning versions of the same operation unless both are actively useful. Extra wrappers make the API harder to learn.

## Errors

Errors should be reported at the layer that understands them.

- Domain parsing should return domain parse errors.
- Persistence should wrap invalid stored data as persistence errors.
- UI should keep invalid draft input local unless the user is trying to commit an operation.
- Application orchestration should translate failures into user-facing status messages.

Avoid panics in ordinary app flow. A panic is acceptable for impossible build-time configuration errors, test setup, or constants that must be valid for the application to start.

## Tests

Tests should follow the same boundaries as the code:

- Domain tests cover parsing, formatting, validation, and pure transformations.
- Persistence tests cover schema validation, round trips, and invalid stored data.
- UI/component tests cover user-visible behavior and state transitions.
- Export tests cover generated output enough to catch regressions in file creation and content.

Prefer small tests that make one rule obvious. Add tests when the behavior could reasonably regress, not merely to exercise every line.

If a test needs a database or UI state to check a simple domain rule, consider moving that rule into a smaller domain function and testing it directly.

## Cleanup Rules

Remove code when it no longer has a real caller or a real product requirement.

Good cleanup candidates:

- Compatibility paths for unsupported old formats.
- Public helpers that should be private implementation details.
- Wrappers that merely duplicate another API.
- Conversion code that serializes a value just to parse it back.
- Tests that only cover deleted or redundant APIs.

Do not clean up by spreading tiny stylistic changes across unrelated modules. Prefer cleanup that makes ownership clearer, removes dead surface area, or simplifies a real workflow.

## Review Checklist

When reviewing a change, ask:

- Are raw strings or integers crossing deeper into the app than they should?
- Is invalid user input kept as draft UI state rather than committed to domain state?
- Does persistence translate rows into domain values at the boundary?
- Are SQL details private to persistence?
- Is mutation happening in the module that owns the state?
- Is a pure helper making the code clearer, or just more indirect?
- Are old compatibility paths intentional?
- Do tests cover the rule at the right layer?
- Would the next maintainer understand the direct path through the code?

Design is not about making every file look the same. It is about making each layer honest about what it owns.
