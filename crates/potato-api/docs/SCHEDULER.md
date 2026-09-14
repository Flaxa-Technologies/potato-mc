# Scheduler & Concurrency Model

Like Minecraft and Paper, PotatoMC's gameplay state (entities, blocks, inventories) must be accessed safely on the main server thread. Long-running tasks (database queries, network requests, file reading) must be dispatched off-thread.

---

## 1. Synchronous Delayed Tasks

Execute a closure on the main server tick thread after a specified delay:

```rust
use std::time::Duration;

let logger = context.logger().clone();

// Executes 5 seconds later
context.scheduler().run_task_later(Duration::from_secs(5), move || {
    logger.info("5 seconds elapsed!");
});
```

---

## 2. Repeating Periodic Tasks

Run a recurring task at fixed intervals:

```rust
let logger = context.logger().clone();

// First run in 10s, repeats every 30s
let task_handle = context.scheduler().run_task_repeating(
    Duration::from_secs(10),
    Duration::from_secs(30),
    move || {
        logger.debug("Server heartbeat check.");
    },
);

// To cancel later:
// task_handle.cancel();
```

---

## 3. Asynchronous Off-Thread Execution

For long-running I/O operations (SQL databases, Redis, Webhooks, HTTP APIs):

```rust
let player_uuid = player.uuid();

std::thread::spawn(move || {
    // 1. Heavy database or HTTP call runs on thread pool
    let stats = fetch_stats_from_db(&player_uuid);

    // 2. Safe to log or process data
    println!("Fetched data: {:?}", stats);
});
```

---

## 4. Best Practices & Rules

1. **Never block the main thread**: Do not perform synchronous disk I/O, heavy computation, or HTTP requests inside event handlers.
2. **Auto-Cleanup**: Tasks registered with `context.scheduler()` are automatically cleaned up if the plugin is disabled or reloaded.
