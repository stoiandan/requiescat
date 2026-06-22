# Functional Programming Know-How

As much as it is feasible, this project should prefer functional programming and pure functions. Ease of reading and simplicity should always prevail. The goal is not to force every part of the code into a theoretical style. The goal is to make ordinary changes easier to reason about, easier to test, and less likely to surprise another part of the system. For broader code design guidance, see [Code Design Know-How](code-design.md).

Pure code is especially valuable in the domain layer, data transformation code, validation logic, and anything that decides what should happen next. Side effects still belong in the system, but they should be explicit and kept near the boundaries that own them.

## Core Principle

Prefer code where the result is determined by the inputs.

A function is easiest to trust when it:

- Takes the data it needs as arguments.
- Returns the result instead of modifying hidden state.
- Does not read from clocks, files, databases, environment variables, global state, or UI state unless that is its explicit job.
- Does not write logs, files, databases, network requests, or UI state unless that is its explicit job.
- Handles errors through return values rather than hidden control flow.

When code has this shape, tests can call it directly. Callers can also compose it without needing to know what else might change behind the scenes.

## Why This Matters Here

Requiescat deals with domain data that should stay understandable: cemeteries, graves, people, relationships, persistence, and UI workflows. These concepts are much easier to maintain when the rules are expressed as transformations over explicit values.

Functional style helps us:

- Keep domain rules independent from storage and UI mechanics.
- Make validation deterministic.
- Make changes easier to test without databases or windows.
- Reduce temporal coupling, where callers must do several steps in the right order.
- Avoid state changes that are hard to trace during UI updates or persistence flows.
- Keep module boundaries honest by making side effects visible.

This does not mean avoiding mutation everywhere. It means choosing mutation deliberately, in places where it makes ownership, performance, or API shape clearer.

## Readability Comes First

Functional style is a tool, not a contest. If a pure transformation, iterator chain, helper function, or abstraction makes the code harder to read than a straightforward mutable block, prefer the straightforward code.

Choose the shape that makes the rule easiest to see:

- Prefer simple control flow over clever composition.
- Prefer direct names over abstract helper names.
- Prefer a small local mutation over a tangled chain of transformations.
- Prefer fewer moving parts when the behavior is already obvious.
- Prefer code that the next maintainer can change confidently.

Pure functions are valuable because they make code clearer and easier to test. When they stop doing that, simplicity wins.

When in doubt, write the boring version first. Extract a pure function only when it gives a real rule a good name, removes meaningful duplication, or makes testing materially easier.

## Prefer Pure Domain Operations

Domain functions should usually accept domain values and return domain values, decisions, or errors.

Prefer this shape:

```rust
fn rename_grave(grave: Grave, new_name: GraveName) -> Result<Grave, DomainError> {
    let name = validate_grave_name(new_name)?;
    Ok(Grave { name, ..grave })
}
```

over this shape:

```rust
fn rename_grave(app: &mut AppState, grave_id: GraveId, new_name: String) {
    let grave = app.repository.load_grave(grave_id).unwrap();
    app.logger.info("renaming grave");
    app.repository.save_grave(grave.with_name(new_name)).unwrap();
    app.refresh_ui();
}
```

The second function may still have to exist somewhere as orchestration, but the domain rule should not be trapped inside it. Pull the rule into a smaller pure function, then let the orchestration code call it.

## Push Side Effects To The Edges

Side effects are necessary. We need to load files, write databases, read the current time, render UI, and report errors. The key is to keep those effects near the modules that own them.

Examples of boundary-owned side effects:

- Persistence modules open database connections, run migrations, and save data.
- UI modules update visible state and react to user input.
- Application orchestration chooses when to load, save, navigate, or notify.
- Infrastructure modules read clocks, paths, configuration, and external resources.

Inside those boundaries, try to convert side-effectful inputs into plain values as soon as possible. Then pass those values into pure domain logic.

## Make Impure Code Thin

When a function must perform side effects, keep the effectful part small and obvious.

A useful pattern is:

```text
load external data
convert it into domain values
call pure functions
save or render the result
```

For example, a repository can be impure internally while still exposing a clean API:

```rust
let cemetery = repository.load()?;
let updated = add_person(cemetery, person)?;
repository.save(&updated)?;
```

The load and save operations are effects. The `add_person` rule should be pure when feasible.

## Avoid Hidden Inputs

Hidden inputs make behavior hard to test and easy to misunderstand.

Avoid domain logic that quietly depends on:

- The current time.
- Randomness.
- Global configuration.
- Process environment.
- Current working directory.
- Static mutable state.
- UI selections or cached screen state.

If a rule needs one of these values, pass it in explicitly:

```rust
fn mark_updated(record: Record, now: DateTime<Utc>) -> Record {
    Record { updated_at: now, ..record }
}
```

This makes the dependency visible, and tests can choose exact values.

## Prefer Values Over Procedures

When practical, represent decisions as values. This keeps the decision separate from the act of carrying it out.

For example, validation can return a result:

```rust
fn validate_cemetery(cemetery: &Cemetery) -> Result<(), ValidationError>
```

A planning function can return an action:

```rust
fn next_persistence_action(state: &AppState) -> PersistenceAction
```

Then a boundary layer can interpret that action and perform the effect. This keeps policy and machinery from collapsing into one large function.

## Use Mutation Intentionally

Rust makes mutation explicit, which is useful. We should still ask why a mutable reference is needed.

Use mutation when it clearly improves:

- Ownership ergonomics.
- Performance for large data structures.
- Integration with APIs that require mutation.
- UI state management.
- Transactional persistence work.

Avoid mutation when it mainly saves a few lines but makes the flow harder to follow. A returned value is often clearer than a function that changes something passed into it.

Prefer:

```rust
let normalized = normalize_person(person)?;
```

over:

```rust
normalize_person_in_place(&mut person)?;
```

unless the in-place operation has a concrete reason to exist.

## Compose Small Functions

Small pure functions compose well. They let us name rules directly and test them without building the whole application around them.

Good candidates for small pure functions:

- Parsing plain values after external data has been read.
- Normalizing names and labels.
- Validating domain invariants.
- Sorting and grouping domain data.
- Deciding whether an operation is allowed.
- Mapping persistence rows into domain values, after the rows are fetched.
- Mapping domain values into persistence records, before the records are written.

Avoid splitting code so finely that the reader has to jump through five files to understand one rule. The point is clarity, not ceremony. A readable inline block is better than a pure helper whose name and call site obscure the actual behavior.

## Keep Error Handling Explicit

Prefer returning `Result` or domain-specific decision types over panics, hidden logging, or partially updated state.

Good error handling should make it clear:

- What failed.
- Which layer owns the failure.
- Whether the caller can recover.
- Whether any side effects already happened.

For pure functions, failures are usually just values. That makes them simple to test:

```rust
let result = validate_grave_name("");
assert_eq!(result, Err(DomainError::EmptyGraveName));
```

## Testing Guidance

Pure functions should have focused tests that cover ordinary cases, edge cases, and invalid inputs. These tests should not need file systems, databases, windows, or clocks.

When testing effectful code:

- Keep the effect under test narrow.
- Use repository traits or module APIs where they already exist.
- Verify that orchestration calls pure logic rather than duplicating rules.
- Avoid testing private implementation details unless they are the only practical seam.

If a test needs a lot of setup for a simple rule, that is often a signal that the rule belongs in a smaller pure function.

## Review Checklist

When reviewing changes, look for opportunities to make logic more functional:

- Is a domain rule mixed into UI or persistence orchestration?
- Could a function return a new value instead of mutating hidden state?
- Are side effects explicit in the function name, arguments, or module boundary?
- Does the function depend on time, randomness, globals, or environment without saying so?
- Could this behavior be tested without a database, file system, or UI?
- Are errors returned as values at the right abstraction level?
- Is mutation justified by ownership, performance, or API constraints?
- Is the functional version actually easier to read than the direct version?

The aim is steady pressure toward clear, deterministic code. Keep the side effects we need, but make the pure center of the system as large and easy to trust as we reasonably can.
