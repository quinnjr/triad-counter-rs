//! TriadCounter - Network triad analysis and counting
//!
//! This plugin analyzes triadic relationships in signed networks based on
//! social balance theory (Easley et al, 2010). It counts triangle configurations
//! and classifies them as stable or unstable.
//!
//! Stable triads:
//! - 3 positive edges (all friends)
//! - 1 positive, 2 negative edges (enemy of my enemy is my friend)
//!
//! Unstable triads:
//! - 2 positive, 1 negative edges (two friends are enemies)
//! - 3 negative edges (all enemies)

use rayon::prelude::*;
use std::path::Path;

/// Results from triad counting analysis
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriadCounts {
    /// Triads with 3 positive edges (all friends)
    pub three_positive: u64,
    /// Triads with 2 positive, 1 negative edge
    pub two_positive: u64,
    /// Triads with 1 positive, 2 negative edges
    pub one_positive: u64,
    /// Triads with 3 negative edges (all enemies)
    pub zero_positive: u64,
}

impl TriadCounts {
    /// Number of stable triads (3 positive or 1 positive)
    #[inline]
    pub fn stable(&self) -> u64 {
        self.three_positive + self.one_positive
    }

    /// Number of unstable triads (2 positive or 0 positive)
    #[inline]
    pub fn unstable(&self) -> u64 {
        self.two_positive + self.zero_positive
    }

    /// Total number of triads
    #[inline]
    pub fn total(&self) -> u64 {
        self.three_positive + self.two_positive + self.one_positive + self.zero_positive
    }

    /// Merge counts from another instance
    #[inline]
    fn merge(&mut self, other: &TriadCounts) {
        self.three_positive += other.three_positive;
        self.two_positive += other.two_positive;
        self.one_positive += other.one_positive;
        self.zero_positive += other.zero_positive;
    }
}

/// TriadCounter plugin for PluMA
pub struct TriadCounterPlugin {
    /// Adjacency matrix (stored as flat vector for cache efficiency)
    adj: Vec<f64>,
    /// Pre-computed sign matrix: 1 = positive, -1 = negative, 0 = zero
    signs: Vec<i8>,
    /// Number of nodes
    n: usize,
    /// Node labels
    labels: Vec<String>,
    /// Computed triad counts
    counts: TriadCounts,
}

impl TriadCounterPlugin {
    /// Create a new empty plugin
    pub fn new() -> Self {
        Self {
            adj: Vec::new(),
            signs: Vec::new(),
            n: 0,
            labels: Vec::new(),
            counts: TriadCounts::default(),
        }
    }

    /// Convert float to sign: 1 = positive, -1 = negative, 0 = zero
    #[inline(always)]
    fn to_sign(v: f64) -> i8 {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    }

    /// Pre-compute sign matrix for fast access
    fn compute_signs(&mut self) {
        self.signs = self.adj.iter().map(|&v| Self::to_sign(v)).collect();
    }

    /// Load adjacency matrix from CSV file
    pub fn input<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(path)?;

        // Get headers (node labels)
        let headers = reader.headers()?.clone();
        self.labels = headers.iter().skip(1).map(|s| s.to_string()).collect();
        self.n = self.labels.len();

        // Pre-allocate adjacency matrix
        self.adj = vec![0.0; self.n * self.n];

        // Read matrix rows
        for (row_idx, result) in reader.records().enumerate() {
            let record = result?;
            for (col_idx, field) in record.iter().skip(1).enumerate() {
                if col_idx < self.n {
                    let value: f64 = field.trim().parse().unwrap_or(0.0);
                    self.adj[row_idx * self.n + col_idx] = value;
                }
            }
        }

        // Zero diagonal
        for i in 0..self.n {
            self.adj[i * self.n + i] = 0.0;
        }

        // Pre-compute signs
        self.compute_signs();

        Ok(())
    }

    /// Count triads - automatically chooses best strategy
    pub fn run(&mut self) {
        if self.signs.is_empty() {
            self.compute_signs();
        }
        self.counts = self.count_triads_optimized();
    }

    /// Optimized triad counting using pre-computed signs
    pub fn count_triads_optimized(&self) -> TriadCounts {
        // Use parallel only for large networks (>500 nodes = 20M+ triads)
        if self.n >= 500 {
            self.count_triads_parallel_chunked()
        } else {
            self.count_triads_sequential()
        }
    }

    /// Sequential triad counting with pre-computed signs
    pub fn count_triads_sequential(&self) -> TriadCounts {
        let mut counts = TriadCounts::default();
        let n = self.n;

        for i in 0..n {
            let i_offset = i * n;
            for j in (i + 1)..n {
                let ij = self.signs[i_offset + j];
                // Skip if no edge between i and j
                if ij == 0 {
                    continue;
                }

                let j_offset = j * n;
                for k in (j + 1)..n {
                    let ik = self.signs[i_offset + k];
                    let jk = self.signs[j_offset + k];

                    // Skip if missing edges
                    if ik == 0 || jk == 0 {
                        continue;
                    }

                    // Count positive edges: sign > 0 gives 1, else 0
                    let pos_count = ((ij > 0) as u8) + ((ik > 0) as u8) + ((jk > 0) as u8);

                    match pos_count {
                        3 => counts.three_positive += 1,
                        2 => counts.two_positive += 1,
                        1 => counts.one_positive += 1,
                        0 => counts.zero_positive += 1,
                        _ => {}
                    }
                }
            }
        }

        counts
    }

    /// Parallel triad counting with chunked workload
    pub fn count_triads_parallel_chunked(&self) -> TriadCounts {
        let n = self.n;

        // Create chunks of approximately equal work
        // Work for row i = sum from j=i+1 to n-1 of (n-1-j) = (n-i-1)(n-i-2)/2
        // We'll parallelize over i with appropriate granularity

        (0..n)
            .into_par_iter()
            .fold(TriadCounts::default, |mut counts, i| {
                let i_offset = i * n;
                for j in (i + 1)..n {
                    let ij = self.signs[i_offset + j];
                    if ij == 0 {
                        continue;
                    }

                    let j_offset = j * n;
                    for k in (j + 1)..n {
                        let ik = self.signs[i_offset + k];
                        let jk = self.signs[j_offset + k];

                        if ik == 0 || jk == 0 {
                            continue;
                        }

                        let pos_count = ((ij > 0) as u8) + ((ik > 0) as u8) + ((jk > 0) as u8);

                        match pos_count {
                            3 => counts.three_positive += 1,
                            2 => counts.two_positive += 1,
                            1 => counts.one_positive += 1,
                            0 => counts.zero_positive += 1,
                            _ => {}
                        }
                    }
                }
                counts
            })
            .reduce(TriadCounts::default, |mut a, b| {
                a.merge(&b);
                a
            })
    }

    /// Write results to output file
    pub fn output<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        writeln!(file, "*********************************************")?;
        writeln!(file, "Stable triads: {}", self.counts.stable())?;
        writeln!(file, "Unstable triads: {}", self.counts.unstable())?;
        writeln!(file)?;
        writeln!(file, "Counts by positive edges:")?;
        writeln!(file, "3: {}", self.counts.three_positive)?;
        writeln!(file, "2: {}", self.counts.two_positive)?;
        writeln!(file, "1: {}", self.counts.one_positive)?;
        writeln!(file, "0: {}", self.counts.zero_positive)?;
        writeln!(file, "*********************************************")?;

        Ok(())
    }

    /// Get the computed triad counts
    pub fn counts(&self) -> &TriadCounts {
        &self.counts
    }

    /// Get number of nodes
    pub fn node_count(&self) -> usize {
        self.n
    }

    /// Get node labels
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Create plugin from adjacency matrix directly (for testing/benchmarking)
    pub fn from_matrix(matrix: Vec<Vec<f64>>) -> Self {
        let n = matrix.len();
        let mut adj = vec![0.0; n * n];

        for (i, row) in matrix.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                if i != j {
                    adj[i * n + j] = val;
                }
            }
        }

        let signs = adj.iter().map(|&v| Self::to_sign(v)).collect();

        Self {
            adj,
            signs,
            n,
            labels: (0..n).map(|i| format!("Node{}", i)).collect(),
            counts: TriadCounts::default(),
        }
    }
}

impl Default for TriadCounterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_all_positive_triad() {
        // 3 nodes, all positive edges -> 1 stable triad (3 positive)
        let matrix = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let mut plugin = TriadCounterPlugin::from_matrix(matrix);
        plugin.run();

        assert_eq!(plugin.counts().three_positive, 1);
        assert_eq!(plugin.counts().stable(), 1);
        assert_eq!(plugin.counts().unstable(), 0);
    }

    #[test]
    fn test_all_negative_triad() {
        // 3 nodes, all negative edges -> 1 unstable triad (0 positive)
        let matrix = vec![
            vec![0.0, -1.0, -1.0],
            vec![-1.0, 0.0, -1.0],
            vec![-1.0, -1.0, 0.0],
        ];
        let mut plugin = TriadCounterPlugin::from_matrix(matrix);
        plugin.run();

        assert_eq!(plugin.counts().zero_positive, 1);
        assert_eq!(plugin.counts().stable(), 0);
        assert_eq!(plugin.counts().unstable(), 1);
    }

    #[test]
    fn test_one_positive_triad() {
        // 3 nodes, 1 positive 2 negative -> 1 stable triad
        let matrix = vec![
            vec![0.0, 1.0, -1.0],
            vec![1.0, 0.0, -1.0],
            vec![-1.0, -1.0, 0.0],
        ];
        let mut plugin = TriadCounterPlugin::from_matrix(matrix);
        plugin.run();

        assert_eq!(plugin.counts().one_positive, 1);
        assert_eq!(plugin.counts().stable(), 1);
    }

    #[test]
    fn test_two_positive_triad() {
        // 3 nodes, 2 positive 1 negative -> 1 unstable triad
        let matrix = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, -1.0],
            vec![1.0, -1.0, 0.0],
        ];
        let mut plugin = TriadCounterPlugin::from_matrix(matrix);
        plugin.run();

        assert_eq!(plugin.counts().two_positive, 1);
        assert_eq!(plugin.counts().unstable(), 1);
    }

    #[test]
    fn test_csv_parsing() {
        let csv = "\"\",A,B,C\nA,0,1,-1\nB,1,0,1\nC,-1,1,0";
        let file = create_test_csv(csv);

        let mut plugin = TriadCounterPlugin::new();
        plugin.input(file.path()).unwrap();

        assert_eq!(plugin.node_count(), 3);
        assert_eq!(plugin.labels(), &["A", "B", "C"]);
    }

    #[test]
    fn test_larger_network() {
        // 4 nodes = C(4,3) = 4 possible triads
        let matrix = vec![
            vec![0.0, 1.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0, 1.0],
            vec![1.0, 1.0, 0.0, 1.0],
            vec![1.0, 1.0, 1.0, 0.0],
        ];
        let mut plugin = TriadCounterPlugin::from_matrix(matrix);
        plugin.run();

        // All positive edges -> all triads have 3 positive
        assert_eq!(plugin.counts().three_positive, 4);
        assert_eq!(plugin.counts().total(), 4);
    }

    #[test]
    fn test_sequential_vs_parallel() {
        // Create a moderate network
        let n = 20;
        let mut matrix = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    matrix[i][j] = if (i + j) % 3 == 0 { -1.0 } else { 1.0 };
                }
            }
        }

        let plugin = TriadCounterPlugin::from_matrix(matrix);
        let seq = plugin.count_triads_sequential();
        let par = plugin.count_triads_parallel_chunked();

        assert_eq!(seq, par);
    }
}
