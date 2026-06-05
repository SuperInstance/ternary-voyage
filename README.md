# ternary-voyage: Long-duration mission planning and execution with ternary progress tracking

## Why This Exists

Agents in a fleet don't just teleport between rooms — they travel through multi-step journeys that take time and consume resources. A deployment pipeline, a multi-room migration, or a cross-cluster data transfer are all voyages: ordered sequences of waypoints with estimated durations and costs.

Standard task trackers treat steps as done or not-done. A voyage needs more nuance: are you ahead of schedule (positive), on track (neutral), or falling behind (negative)? This ternary status lets the fleet adjust in real time — reroute around obstacles, allocate more resources to lagging voyages, or abort missions that can't recover.

## Core Concepts

- **Ternary status**: `Neg` (behind schedule), `Zero` (on track), `Pos` (ahead of schedule). Computed by comparing progress fraction to resource consumption fraction.
- **Voyage**: A multi-waypoint journey with a resource budget. Starts as Planned, transitions to Active, and ends as Completed or Aborted.
- **Waypoint**: A stop along the voyage with an expected duration and resource cost. Waypoints are ordered and reached sequentially.
- **VoyageLog**: A timestamped event log for recording what happened during the voyage.
- **VoyageEstimator**: Predicts remaining time and resources based on unreached waypoints. Reports ternary status.
- **VoyageNavigator**: Adjusts the voyage plan mid-flight — insert, remove, or reroute waypoints (only unreached ones can be changed).
- **VoyageCompletion**: Defines success criteria (required waypoints + minimum remaining resources) and verifies the voyage met them.

## Quick Start

```toml
[dependencies]
ternary-voyage = "0.1"
```

```rust
use ternary_voyage::*;

let mut voyage = Voyage::new(1, "deploy-to-east", 1000);
voyage.add_waypoint(Waypoint::new(1, "build", 30, 100));
voyage.add_waypoint(Waypoint::new(2, "test", 60, 200));
voyage.add_waypoint(Waypoint::new(3, "deploy", 20, 300));

voyage.start();
voyage.consume_resources(100);
voyage.advance(); // reached "build"

let remaining = VoyageEstimator::estimate_remaining_resources(&voyage);
println!("Still need {} resource units", remaining);

let status = VoyageEstimator::status(&voyage);
// If progress > resource usage → Ternary::Pos (ahead)
```

## API Overview

| Type | Description |
|------|-------------|
| `Voyage` | Multi-step journey with waypoints, resources, and status tracking |
| `Waypoint` | A stop with expected duration, resource cost, and reached flag |
| `VoyageLog` | Timestamped event log for voyage history |
| `VoyageEstimator` | Predicts remaining time/resources and reports ternary status |
| `VoyageNavigator` | Modifies the voyage plan (insert, remove, reroute waypoints) |
| `VoyageCompletion` | Defines and verifies mission success criteria |
| `CompletionResult` | Verification outcome with missing waypoints and resource status |

## How It Works

A voyage is an ordered list of waypoints. When started, the first waypoint becomes current. Each call to `advance()` marks the current waypoint as reached and moves to the next. When the last waypoint is advanced past, the voyage automatically completes.

The estimator compares progress (fraction of waypoints reached) against resource consumption (fraction of total budget used). If progress exceeds consumption by more than 10%, the status is `Pos` (ahead). If consumption exceeds progress by more than 10%, it's `Neg` (behind). Within the 10% band, it's `Zero` (on track).

The navigator can modify the waypoint list but only unreached waypoints — you can't change history. This means course corrections are always forward-looking. Inserting a waypoint after the current one adds a new stop; removing an unreached one skips it; rerouting replaces an unreached waypoint entirely.

Completion verification checks that all required waypoints were reached and that remaining resources meet the minimum threshold. A voyage can be "completed" (all waypoints reached) but still fail completion criteria if resources are too low.

## Known Limitations

- Waypoints are strictly sequential; no branching or parallel paths.
- The 10% threshold for status classification is hardcoded, not configurable.
- Time estimates are based on waypoint `expected_duration` sums — no actual clock integration.
- No support for dependent waypoints (where one can't start until another finishes).
- Resource consumption is manual — the caller must call `consume_resources()` at appropriate times.
- No persistence or serialization; voyage state exists only in memory.

## Use Cases

- **Deployment pipelines**: Model a multi-stage deploy (build → test → stage → prod) as a voyage with resource budgets.
- **Agent migration**: Plan an agent's journey through multiple rooms with estimated transfer times.
- **Long-running experiments**: Track progress through experiment phases with resource accounting.
- **Fleet logistics**: Coordinate multi-agent missions where each agent follows its own voyage plan.

## Ecosystem Context

Part of the SuperInstance ternary crate family. Works with `ternary-mesh` (network connectivity between voyage waypoints) and `ternary-quorum` (for deciding whether to approve or abort a voyage). The voyage handles the execution timeline; the mesh handles the transport; the quorum handles the governance.

## License

MIT

## See Also
- **ternary-navigator** — related fleet coordination
- **ternary-compass** — related fleet coordination
- **ternary-harbor** — related fleet coordination
- **ternary-anchor** — related fleet coordination
- **ternary-beacon** — related fleet coordination
- **ternary-observatory** — related fleet coordination

