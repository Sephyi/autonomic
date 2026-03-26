# Rust 1.94 Features for Autonomic

Use these features freely throughout the codebase. Edition 2024, MSRV 1.94.

## Let Chains (1.88)

Combine multiple `let` bindings in conditions:

```rust
if let Some(config) = load_config() && let Ok(pool) = connect_db(&config) {
    run_server(pool).await;
}
```

## Generic Argument Inference (1.89)

Let the compiler infer array lengths:

```rust
let buffer: [u8; _] = [0; _]; // compiler infers size from usage
```

## LazyLock::get (1.94)

Check initialization state without triggering:

```rust
use std::sync::LazyLock;

static DB_POOL: LazyLock<PgPool> = LazyLock::new(|| { /* ... */ });

// Check if pool is initialized without creating it
if LazyLock::get(&DB_POOL).is_some() {
    // Pool already initialized
}
```

## Async Closures (1.85)

Use `async || {}` with `AsyncFn` trait — no boxed futures for callbacks:

```rust
let process = async |trace: ExperienceTrace| {
    store.insert(trace).await?;
    Ok(())
};
```

## Precise Capturing `use<>` (1.87)

Control what an `impl Trait` return type captures:

```rust
fn query_memory<'a>(&'a self, keywords: &str) -> impl Future<Output = Vec<MemoryEntry>> + use<'a, Self> {
    async move { self.store.search(keywords).await }
}
```

## Trait Upcasting (1.86)

Coerce trait objects to supertrait objects:

```rust
trait ContainerRuntime: Send + Sync { /* ... */ }
trait PodmanRuntime: ContainerRuntime { /* ... */ }

// Arc<dyn PodmanRuntime> coerces to Arc<dyn ContainerRuntime>
let runtime: Arc<dyn ContainerRuntime> = podman_runtime;
```

## Safe `#[target_feature]` (1.86)

Use platform-specific features without unsafe:

```rust
#[target_feature(enable = "neon")]
fn fast_hash(data: &[u8]) -> u64 {
    // NEON intrinsics callable from safe code
    todo!()
}
```

## `get_disjoint_mut` (1.86)

Safe concurrent access to multiple map entries:

```rust
use std::collections::HashMap;

let mut map: HashMap<&str, Vec<String>> = HashMap::new();
if let Ok([a, b]) = map.get_disjoint_mut(["traces", "metrics"]) {
    a.push("trace1".into());
    b.push("metric1".into());
}
```

## `Vec::extract_if` (1.87)

Drain matching elements:

```rust
let expired: Vec<_> = variants.extract_if(.., |v| v.is_expired()).collect();
```

## `HashMap::extract_if` (1.88)

Drain matching entries:

```rust
let stale: Vec<_> = memory_entries.extract_if(|_, entry| entry.relevance() < 0.01).collect();
```

## RPIT Captures All In-Scope Lifetimes (Edition 2024)

No more `+ '_` needed on return position impl Trait — all in-scope lifetimes are captured by default.
