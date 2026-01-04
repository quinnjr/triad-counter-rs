# triad-counter-rs

Rust implementation of the TriadCounter plugin for PluMA - Network triad analysis based on social balance theory.

## Overview

This plugin analyzes triadic relationships in signed networks (Easley et al, 2010). It counts triangle configurations and classifies them as stable or unstable according to social balance theory.

### Social Balance Theory

- **Stable triads:**
  - 3 positive edges (all friends)
  - 1 positive, 2 negative edges (enemy of my enemy is my friend)

- **Unstable triads:**
  - 2 positive, 1 negative edges (two friends are enemies)
  - 3 negative edges (all enemies)

## Installation

```bash
cargo install --path .
```

## Usage

### Command Line

```bash
triad-counter input.csv output.txt
```

### Input Format

CSV adjacency matrix with node labels:

```csv
"",A,B,C
A,0,1,-1
B,1,0,1
C,-1,1,0
```

- Positive values indicate positive relationships
- Negative values indicate negative relationships
- Diagonal is ignored (self-loops)

### Output Format

```
*********************************************
Stable triads: 5
Unstable triads: 3

Counts by positive edges:
3: 2
2: 1
1: 3
0: 2
*********************************************
```

### As a Library

```rust
use triad_counter_rs::TriadCounterPlugin;

let mut plugin = TriadCounterPlugin::new();
plugin.input("network.csv")?;
plugin.run();

let counts = plugin.counts();
println!("Stable: {}, Unstable: {}", counts.stable(), counts.unstable());

plugin.output("results.txt")?;
```

## Performance

The implementation uses:
- Flat adjacency matrix for cache-efficient access
- Parallel processing with rayon for networks > 50 nodes
- SIMD-friendly edge sign classification

Benchmarks show significant speedup over the Python implementation, especially for larger networks.

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Benchmarking

```bash
cargo bench
```

## References

- Easley, D., & Kleinberg, J. (2010). Networks, Crowds, and Markets: Reasoning About a Highly Connected World. Cambridge University Press.
- Original PluMA plugin: https://github.com/movingpictures83/TriadCounter

## License

MIT
