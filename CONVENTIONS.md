# Conventions

How the code in this repository is written. ARCHITECTURE.md says what the parts are; this file
says how to write them. If a rule here blocks you, say so rather than deviating quietly.

## 1. Design style

### Prefer pure functions

A function that takes values and returns values can be tested with a table, reasoned about in
isolation, and reused from a context its author never imagined. One that reaches for a socket,
a clock or a global cannot. Push effects to the edges: a reducer returns the effects it wants
(`reduce_*` returns state and a list of effects; the service's live loop carries them out), and
a machine returns what the component should do next rather than doing it.

Effects belong in the signature. `&mut self`, `Result`, or living in a module allowed to do I/O
are how a reader sees that something happens; a function that looks pure and is not is worse
than one that is honestly imperative.

### Separate data from logic

Data types describe what is true; functions describe what follows from it. A type's own `impl`
holds construction, derivation and accessors. Anything that coordinates several values, or
makes a decision, reads better as a free function. If half a struct's methods never touch half
its fields, it is two types.

### Make illegal states unrepresentable

- Newtype every identifier and every unit (`AppId`, `WindowKey`, `Percent`, `Cpx`). A newtype
  costs nothing at runtime and makes an argument-order mistake a compile error.
- No stringly-typed fields where a closed set exists.
- No `bool` in state, in a public signature or in a settings key; two-state enums with named
  variants (`Presence::{Present, Absent}`, `Availability::{Enabled, Disabled}`). A call reading
  `send(true)` tells the reader nothing; `send(Confirm::Yes)` tells them everything. Predicate
  returns are `bool`.
- Prefer an empty collection to `Option<Vec<T>>`.
- Model absence precisely.

### Parse, don't validate

Convert untrusted input into a type that cannot be wrong, once, at the boundary (a D-Bus
message, a desktop entry, an `.ics` file, a settings value), and the rest of the program needs
no defensive checks. A fallible constructor returns `Result`; one that cannot fail returns
`Self`.

### Traits are seams, not decoration

A trait earns its place when two implementations are genuinely swapped at that boundary: a real
one and a fake a test drives (`Authenticator`, `DdcBus`, `FileIndex`). A trait with one
implementation is indirection with no payoff. Prefer an enum for a closed vocabulary, so adding
a variant produces a compile error at every site that must handle it. Test seams are injected
values, never an environment variable read deep inside a service.

### Errors are values

One error enum per crate or service, built with `thiserror`. An error exists so a caller can act.
Panics are for broken invariants in our own code; malformed input from a bus, a file or the
compositor is never a panic. No `unwrap()` or `expect()` outside tests unless a comment on the
same line proves the invariant.

### Composition over accumulation

Iterator chains where they read more clearly than a loop, a plain loop where they do not. Prefer
returning a new value to mutating an argument; keep a mutable accumulator local. A long function
is usually several functions that have not been named yet.

## 2. Derives

Public types derive `Debug` and whatever else genuinely holds: `Clone`, `PartialEq`, `Eq`;
`Copy` only for types a word or smaller; `Hash` for identifiers and map keys; `PartialOrd`,
`Ord` only where an order means something; `Default` only where a default is meaningful. Derive
rather than hand-write, except for secrets: a type holding a password or a clipboard entry
writes its `Debug` by hand and redacts (`Secret(..)`, an excerpt's length only).

## 3. Serde

- Enums with data: adjacently tagged, `#[serde(tag = "kind", content = "v", rename_all =
  "snake_case")]`. Fieldless enums: `rename_all = "snake_case"`.
- Nothing is released, so there is no compatibility to keep: no aliases for old names, no
  migrations, no fields kept only so an old file still loads. An unknown key in a file sill
  reads is reported as a load diagnostic, not preserved.
- Every IPC wire form is pinned by a test (`sill-ipc/tests/serde.rs`).

## 4. Modules and files

One concept per file; a file near 400 lines wants splitting. `lib.rs` and `mod.rs` declare
modules and re-export; they hold no logic. Public items carry a doc comment saying what they
mean, not what they are.

## 5. Tests

- Unit tests in `#[cfg(test)] mod tests` beside the code (or a sibling `*_tests.rs`), for private
  behaviour; integration tests in `crates/<crate>/tests/<topic>.rs`.
- Table-driven wherever there is more than one case; a failure names its case.
- Assert the difference the action makes, not a state it happens to be compatible with: read
  the quantity before, act, compare. An assertion that could be moved above the line doing the
  work and still pass proves nothing. Ask the program for the thing under test rather than
  restating it in the test.
- Substrings are not tokens: a domain, a capability, a status code and a word are compared at
  their boundaries, never with `contains`/`ends_with`.
- Before concluding something does not work, run it the way its user would, with nothing of
  yours around it. A failure that reproduces only through your own harness is a fact about the
  harness.

## 6. Verification

Before reporting work complete, the gate (`~/sill-wt/sill-gate.sh`, or its steps by hand) must
pass, checking every exit code rather than a piped summary:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

`scripts/check-boundary.sh` checks for output, not exit status: `cargo tree -i <dep>` exits 101
when the dependency is absent, which is the state wanted.

Report honestly: a failing test reported as passing costs more than the bug did.

## 7. Borrowing from other projects

sill is `MIT OR Apache-2.0`. Permissive code (MIT, Apache-2.0, BSD) may be copied with
attribution. Weak copyleft (MPL, LGPL) may be linked, never pasted: MPL's unit is the file.
Strong copyleft (GPL: cosmic-comp, cosmic-greeter, cosmic-idle, cosmic-osd, systemd) is read for
facts only: note what it does in a sentence about observable behaviour, close the file, write
from the sentence. Never put GPL source into a model's context and ask for an equivalent. Cite
the behaviour, not the source line.

## 8. sill

- **Crate boundaries** (`scripts/check-boundary.sh`): `sill-ipc` and `sill-launcher` are pure
  (no dioxus, zbus, wayland-client, tokio or libpulse); `sill-services` is effectful but UI-free
  (no dioxus, blitz, shell-host, ds or ds-settings). UI reaches services only through watch
  channels read by `sill-surfaces`' hooks.
- **A service** is `spawn(rt, seams) -> (watch::Receiver<State>, Control<Command>)` plus a
  pure reducer; its seams (buses, compositor, directories, clock, DDC, session actions) come
  from `sill_services::env::Env` and nothing else: no environment variable, `dirs::*` call or
  hard-coded bus inside a service. A service whose system half is missing still yields its
  receiver, at its default. A service that could touch the person's session (notifications,
  idle, polkit, clipboard, calendar, screenshots) is passive until the daemon sends its first
  policy, and a test's `Services::spawn(rt, &Env::isolated(scratch))` reaches no bus,
  compositor or real file.
- **Never pass a Dioxus signal through `Root::context`**; roots get `Services`, `SettingsFeed`
  and plain links (watch channels).
- **Design system rules** (quire): every class is `ds-`; consumers never style `.ds-*` or
  `[data-theme|accent|motion|material]`; every colour, duration, easing, keyframe and font comes
  from quire's tokens; no raw `button`/`input`/menu markup. A missing component or token is
  added to quire first, never patched locally. Motion is quire's (`ds::detail`, springs, timers);
  sill writes no keyframes.
- **Every time-based UI test runs on `Clock::Virtual`** and pins `PinnedAppearance`; production
  time goes through `ds::time`.
- **Every value the design docs mark "proposed" is read from a settings key** with that default.
  A new key is a design/22 row in quire first; `sill-settings/tests/keys.rs` cross-checks every
  key against quire's rows, both directions.
- **No `unsafe`** (`unsafe_code = "deny"` at the workspace). No `std::env::set_var` (unsafe since
  edition 2024): make the environment read injectable instead.
- **Dependencies** in the pinned block of the root `Cargo.toml` are copied from quire's
  `docs/workspace-deps.toml` and changed only there; sill-only dependencies sit after the block.
- **Tests never touch the real system.** No test or script reaches the real session or system
  bus, NetworkManager, UPower, logind, BlueZ, `/dev/i2c-*`, the real `~/.config`, or the
  person's Wayland session. Live tests (`*_live.rs`) are `#[ignore]`d and use
  `tests/support/private_bus.rs`. Scripts give the daemon a private bus on both addresses, a
  fake DDC, scratch XDG dirs and its own `XDG_RUNTIME_DIR` (`dev/common.sh`), and run on the
  nested cosmic-comp or shell-host's headless KWin, never the person's session. No test locks a
  session it did not start, calls logind's `Suspend`, or registers a polkit agent outside a
  private bus.
