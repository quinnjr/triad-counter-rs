//! TriadCounter CLI - Network triad analysis tool
//!
//! Usage: triad-counter <input.csv> <output.txt>

use std::env;
use std::process;
use triad_counter_rs::TriadCounterPlugin;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.csv> <output.txt>", args[0]);
        eprintln!();
        eprintln!("Analyzes triadic relationships in signed networks.");
        eprintln!("Input: CSV adjacency matrix with node labels");
        eprintln!("Output: Triad counts and stability analysis");
        process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    let mut plugin = TriadCounterPlugin::new();

    // Input phase
    if let Err(e) = plugin.input(input_file) {
        eprintln!("Error reading input file '{}': {}", input_file, e);
        process::exit(1);
    }

    eprintln!(
        "Loaded network with {} nodes ({} possible triads)",
        plugin.node_count(),
        count_triads(plugin.node_count())
    );

    // Run phase
    plugin.run();

    let counts = plugin.counts();
    eprintln!(
        "Found {} triads: {} stable, {} unstable",
        counts.total(),
        counts.stable(),
        counts.unstable()
    );

    // Output phase
    if let Err(e) = plugin.output(output_file) {
        eprintln!("Error writing output file '{}': {}", output_file, e);
        process::exit(1);
    }

    eprintln!("Results written to '{}'", output_file);
}

/// Calculate number of possible triads: C(n, 3) = n! / (3! * (n-3)!)
fn count_triads(n: usize) -> usize {
    if n < 3 {
        0
    } else {
        n * (n - 1) * (n - 2) / 6
    }
}
