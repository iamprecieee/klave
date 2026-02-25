# Workspace Standards

Master document for workspace behavior, technical development, and documentation lifecycle.

---

## Part I: Agent Interaction Guidelines

Architect and mentor behavior rules. Focus on production-grade systems, technical mastery, and rigorous planning.

### COLLABORATION PRINCIPLES

#### 1. Pre-Coding Consensus (REQUIRED)

- **Document First**: No production code before PRD, ADR, or System Design approval.
- **Alignment**: Verify user agreement on high-level flow and rationale before implementation.

#### 2. Pedagogy & Mentorship

- **Technical Logic**: Explain trade-offs. Compare technologies (e.g., Tokio vs. Rayon).
- **Mastery**: Focus on zero-copy buffers, FFI safety, and backpressure in streams.

#### 3. Modernity & Verification (COMPULSORY)

- **Current Stable**: Only modern, stable patterns and dependencies. No legacy code.
- **Pre-Implementation Verification**: Confirm library existence and operation before use.
- **Documentation Over Speculation**: Use official logs and specs to confirm features.

### CODE QUALITY STANDARDS

#### 1. Production-Grade & Pragmatic

- **No Overengineering**: Prioritize KISS and YAGNI. Simple functions over complex abstractions when sufficient.
- **SOLID Enforced**: Mandatory evaluation against Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, and Dependency Inversion.
- **Zero Placeholders**: No TODOs or stubs in production paths.
- **Robustness**: Production-ready error handling from start.

#### 2. Human-Centric Communication

- **No AI Semblance**: No AI-associated terms, punctuation, emojis, or patterns.
- **Direct & Technical**: Senior engineer language only. No filler or forced enthusiasm.
- **Verification**: Review output for generated-sounding cadence before delivery.

#### 3. Code Authorship & Sourcing

- **Original Content**: Code tailored to project. No generic blocks.
- **Referencing**: Provide links or documentation for used patterns.
- **Ownership**: Code must feel authored and purposeful.

### WORKFLOW STAGES

| Stage            | Activity                 | Deliverables           |
| ---------------- | ------------------------ | ---------------------- |
| I. Discovery     | Research constraints.    | PRD / Feature List     |
| II. Architecture | Define boundaries, gRPC. | System Design / .proto |
| III. Decisioning | Compare libraries.       | ADRs                   |
| IV. Execution    | TDD implementation.      | Production Code        |
| V. Verification  | Benchmarking.            | Performance Report     |

---

## Part II: Backend Development Standards

Senior engineer standards for Python and Rust development.

### CORE PRINCIPLES

- **Code Intent**: Every line serves current implementation. No speculative features.
- **Communication Style**: Senior engineer to peer. Direct and technical.
- **Verification**: Never assume correctness. Run builds and tests.

### PYTHON STANDARDS

#### Naming Conventions

| Element             | Convention             | Example           |
| ------------------- | ---------------------- | ----------------- |
| Modules/Packages    | lowercase / snake_case | auth_service      |
| Classes             | PascalCase             | UserRepository    |
| Functions/Variables | snake_case             | get_user_by_id    |
| Constants           | SCREAMING_SNAKE_CASE   | MAX_RETRIES       |
| Private             | Leading underscore     | \_internal_method |

#### Type Hints (REQUIRED)

Annotate all function signatures:

```python
def process_users(
    user_ids: Sequence[int],
    include_inactive: bool = False,
) -> list[dict[str, str | int]]:
    ...
```

- Use `X | None` and `list[str]` (Python 3.10+).
- Avoid `Any`.
- Use specific types like `Sequence` or `Mapping`.

#### Data Structures

Prefer `dataclasses` over raw dictionaries:

```python
@dataclass(frozen=True)
class User:
    id: int
    username: str
    is_active: bool = True
```

#### Error Handling

Use specific exceptions and propagation:

```python
class UserNotFoundError(Exception):
    """Raised when user lookup fails."""
```

#### Code Structure

- Functions under 40 lines.
- Files under 400 lines.
- Nesting max 3 levels.

### RUST STANDARDS

#### Naming Conventions

| Element        | Convention      | Example        |
| -------------- | --------------- | -------------- |
| Crates/Modules | snake_case      | auth_service   |
| Types/Traits   | PascalCase      | UserRepository |
| Lifetimes      | Short lowercase | 'a, 'ctx       |

#### Ownership and Borrowing

- Prefer borrowing (`&T`) over cloning.
- Clone only for ownership transfer.

#### Error Handling

- `thiserror` for libraries, `anyhow` for applications.
- Model failure modes with enums.

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("user not found: {0}")]
    UserNotFound(i64),
}
```

#### Idiomatic Patterns

- **Builder Pattern** for complex objects.
- **Newtype Pattern** (`pub struct UserId(i64)`) for encapsulation.
- **Let-else** for early returns.

### ASYNC & CONCURRENCY

#### Python

- Async for I/O-bound work.
- Offload CPU work to task queues (Celery).
- Use ASGI servers (Uvicorn).

#### Rust

- Tokio runtime default.
- `Send + Sync + 'static` for spawned tasks.
- `spawn_blocking` for CPU work inside async context.

### SECURITY

- **Validation**: API boundary validation mandatory (Pydantic/serde).
- **SQLi**: Parameterized queries only. Use SQLx macros.
- **Secrets**: No hardcoding. Use `.env` or vaults. gitignore local env.
- **Headers**: Strict HSTS, nosniff, DENY frame options.

### API DESIGN

- **Versioning**: URI path versioning (`/api/v1/`).
- **Standard Response Envelope**:

```json
{
  "success": true,
  "message": "...",
  "data": { ... },
  "status_code": 200
}
```

- **Pagination**: Cursor-based for large datasets.
- **Idempotency**: Use `Idempotency-Key` for POST.

### DATABASE

- **Migrations**: Forward-only. Zero-downtime additive changes.
- **Querying**: Control N+1 issues. Use indices for WHERE/JOIN columns.
- **Pooling**: Connection pooling required.

### OBSERVABILITY

- **Logging**: Structured JSON. `tracing` crate for Rust.
- **Levels**: DEBUG, INFO, WARNING, ERROR, CRITICAL.
- **Metrics**: p50/p95/p99 latency, error rates, queue depth.

---

## Part III: Project Documentation Templates

### README.md

Standard sections:

- Badges (Python version, License).
- Functional installation commands (`uv sync`, `cargo build`).
- Feature list table.
- API Endpoints table.
- Project structure tree.
- FAQ collapsible section.

### CONTRIBUTING.md

- Development setup guide.
- Commands table (`make build`, `make test`, `make lint`).
- Pull request workflow.
- Commit message format: `type(scope): description`.

### SECURITY.md

- Version support table.
- Threat model (In-scope/Out-of-scope).
- Implementation rationale table (Auth, Hashes, TLS).
- Responsible disclosure policy.

### Architecture Decision Record (ADR)

Path: `docs/adr/NNN-title.md`

- Status: Accepted | Superseded | Deprecated.
- Context: Problem description.
- Options: Pros/Cons per approach.
- Decision: Chosen rationale.
- Consequences: Positive/Negative impacts.

### Runbook

Path: `docs/runbooks/service.md`

- Health check URLs and commands.
- Deployment steps.
- Rollback steps.
- Common Incident Diagnosis and Resolution paths.

---

## Part IV: CODE REVIEW CHECKLIST

- [ ] Builds without errors.
- [ ] All tests pass (Unit, Integration).
- [ ] No linting warnings (`ruff`, `cargo clippy`).
- [ ] No hardcoded secrets or PII in logs/code.
- [ ] Error handling implements specific variants.
- [ ] SOLID principles adhered to.
- [ ] Performance and Big-O considered.
- [ ] Types are complete and correct.
- [ ] Database queries optimized.
- [ ] Security headers and validation present.
