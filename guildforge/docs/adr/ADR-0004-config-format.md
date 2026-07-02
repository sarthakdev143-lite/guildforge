# ADR-0004: Config Format (YAML v1, No Modules)

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: config, yaml, schema

## Context

GuildForge needs a config format that:

1. **Declaratively** describes the desired state of a Discord guild.
2. Is **human-readable** and **git-friendly** (clean diffs).
3. Is **strongly typed** (no `HashMap<String, Value>`).
4. **Validates** at parse time and at semantic time, with precise
   diagnostics pointing to file:line:col.
5. Is **familiar** to the target audience (DevOps engineers who already
   know YAML from Kubernetes, Ansible, GitHub Actions).
6. Is **extensible** — we can add new resource types without breaking
   existing configs.

The format choice is hard to reverse. Once users have committed YAML
files to git, switching formats is a major version bump and a migration
tool. We need to pick correctly now.

## Decision

### Format: YAML 1.2 (strict subset)

- File extension: `.yaml` or `.yml`.
- Encoding: UTF-8.
- **Strict deserialization**: unknown keys are errors, not silent drops
  (`#[serde(deny_unknown_fields)]` on every struct).
- **No anchors / aliases / merges**. Forbidden in v1. They break
  git-friendliness (a rename refactors everywhere) and confuse reviewers.
- **No YAML tags** (`!!str`, `!!python/object`). Forbidden. `serde_yaml`
  doesn't support them anyway.
- **No multi-document YAML** (`---` separators). One config = one file.
- **Single root mapping**. Top level is always a mapping with the keys
  defined in [`docs/SCHEMA.md`](../SCHEMA.md).

### No modules in v1

v1 configs are **single-file**. There is no `module`, `import`, or
`include` directive. A guild = a file.

Reasons:

- Modules add a layer of complexity (variable passing, output exporting,
  versioning, registry) that v1 doesn't need.
- A typical guild fits comfortably in a single 200-500 line YAML file.
- Modules make `validate` and `plan` I/O-bound (must read every module);
  in v1 they're pure functions of one file + state.
- Terraform's module system took 5+ years to mature. We don't need it on
  day one.

Modules are deferred to Phase 7+ and will require their own ADR.

### No variables in v1

v1 has no `variable`, `local`, or `output` blocks. Configs are static.

Reasons:

- Variables require a type system, a templating story, default values,
  validation rules. Each is a rabbit hole.
- For 90% of guilds, the YAML is written once and edited rarely. Static
  is fine.
- For the 10% that need parameterization (e.g. deploying the same shape
  to dev / staging / prod), users can use `envsubst` or a Makefile
  pre-processor. We don't need to ship a templating engine.

When we do add variables (Phase 7+), they will use a syntax like:

```yaml
variables:
  - name: env
    type: string
    default: dev

roles:
  - name: Admin-${env}
```

…which is a self-contained ADR.

### Strong typing via serde

Every YAML structure maps to a Rust struct. No `HashMap<String, Value>`,
no `serde_yaml::Value`, no dynamic dispatch. The full type tree is in
`crates/config/src/lib.rs` and documented in
[`docs/CRATE_LAYOUT.md`](../CRATE_LAYOUT.md).

Benefits:

- Compile-time guarantee that every field is handled.
- `cargo doc` generates schema documentation for free.
- Refactors that change the schema break compilation, not runtime.
- IDE support: rust-analyzer completes field names.

### Validation is split

- **Syntactic validation** (YAML parse + serde deserialize): in
  `crates/parser`. Produces `ParseError` with span.
- **Semantic validation** (references resolve, no dupes, API limits
  respected): in `crates/validation`. Produces `Vec<Diagnostic>` with
  stable codes (V001-V075).

Splitting these lets us produce ALL semantic errors in one pass (users
see every problem, not just the first) and lets the parser stay simple
(serde does the work).

### Schema versioning

Optional top-level `_schema_version: 1` key. If present and > 1, parser
rejects with `ParseError::UnsupportedSchemaVersion`. If absent, v1 is
assumed.

Future v2 schemas ship with a `guildforge migrate` command that
transforms v1 files to v2 syntax. v1 support is maintained for one
major GuildForge version after v2 ships.

### Stable export

`guildforge export` produces YAML in a canonical form (see SCHEMA.md §6).
Two exports of the same state produce byte-identical files. This makes
exported YAML diff cleanly in git.

### Examples and templates

- `examples/` — runnable, validated example configs (commitment: every
  example passes `guildforge validate`).
- `templates/` — opinionated starter configs for common guild shapes
  (minimal, community, company, opensource-project). Used by
  `guildforge init --template <name>`.

## Alternatives Considered

### D1: HCL (HashiCorp Configuration Language)

Rejected. Pros: familiar to Terraform users, native expression support.
Cons: no mature Rust parser; we'd write one. HCL's expression semantics
force a type system on day one. The familiar-to-Terraform-users argument
is weaker than it sounds — most DevOps engineers know YAML better than
HCL because of Kubernetes / Ansible / GitHub Actions.

### D2: TOML

Rejected. TOML's table syntax doesn't scale to lists of nested resources
(`[[channels]]` × 100 is painful to read). TOML is great for
configuration of single programs (Cargo.toml) but bad for declarative
resource catalogs.

### D3: JSON

Rejected. JSON is valid YAML, so YAML users get JSON for free. JSON is
harder to read (no comments, verbose quoting) and harder to write by
hand. We accept JSON input (serde_json can deserialize our types) but
don't promote it.

### D4: JSON-with-schema (JSON Schema + any JSON)

Rejected. JSON Schema is verbose and we'd be writing the schema twice
(JSON Schema + Rust types). With serde + deny_unknown_fields, the Rust
types ARE the schema.

### D5: Dhall / Cue / Jsonnet

Rejected. All three are excellent configuration languages with strong
type systems. None has mainstream adoption; requiring users to learn a
new language to use GuildForge is a high bar. If we ever add a templating
layer (Phase 7+), CUE is the most likely candidate (it's designed for
exactly this use case and has good Rust bindings).

### D6: Modules in v1

Rejected. See "No modules in v1" above. Adding modules later is a
strictly additive change (new `modules:` top-level key); it does not
break existing configs.

## Consequences

### Becomes easier

- Parser is ~200 lines of `serde_yaml` calls. Trivial.
- Validation is a pure function over `Config`. Easy to test.
- IDE support: rust-analyzer completes field names in Rust source. YAML
  itself gets schema completion via JSON Schema export (future work).
- `guildforge export` is canonical: byte-stable across runs.
- Examples in `examples/` are validated in CI — they always work.

### Becomes harder

- No variables means CI pipelines must use `envsubst` or similar. We
  document this pattern in `docs/COOKBOOK.md` (TBD).
- No modules means multi-guild orgs duplicate config. We accept this for
  v1; Phase 7+ addresses it.
- Unknown keys are errors, so adding a new optional field is a
  non-breaking change (serde defaults it to `None`) but adding a new
  required field is a breaking change. We commit to never adding
  required fields to existing structs after v1.0.

### New constraints

- Every struct has `#[serde(deny_unknown_fields)]`. Code review enforces.
- Every optional field is `Option<T>` with `skip_serializing_if =
  "Option::is_none"`. This is what makes `export` byte-stable.
- Field order in structs is the order they appear in exported YAML.
  Reordering fields is a breaking change for `export` output.
- Adding a new resource kind = new `Resource` variant + new serde model
  + new validator rules. All three in the same PR.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Users want variables before Phase 7 | Document `envsubst` pattern; common need is small |
| YAML ambiguity (Norway problem, implicit typing) | Strict typing via serde + `deny_unknown_fields`; quote strings that could be misinterpreted |
| Users copy-paste configs and diverge | Future module system; for v1, `diff` command helps |
| Schema adds required field, breaks existing configs | Policy: never add required fields after v1.0; new fields are always optional |
| Anchor/alias demand from users | Defer to ADR if demand materializes; not in v1 |

## References

- [YAML 1.2 spec](https://yaml.org/spec/1.2.2/)
- [serde deny_unknown_fields](https://serde.rs/attr-denied-unknown-fields.html)
- [Terraform HCL](https://github.com/hashicorp/hcl)
- [CUE](https://cuelang.org/)
- Related: [ADR-0005](./ADR-0005-error-model.md) (diagnostics with spans),
  [`docs/SCHEMA.md`](../SCHEMA.md) (full schema)
