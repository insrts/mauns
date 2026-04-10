//! Unified-diff generation for file changes.
//!
//! Produces a minimal unified diff string in the standard `--- / +++` format
//! without any external diffing library dependency.

/// Generate a unified diff between `old` and `new` content.
///
/// The header lines use `path` as the file label.
/// Returns an empty string when the contents are identical.
pub fn unified_diff(path: &str, old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let hunks = compute_hunks(&old_lines, &new_lines);
    if hunks.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&format!("--- {path}\n"));
    out.push_str(&format!("+++ {path}\n"));

    for hunk in hunks {
        out.push_str(&hunk);
    }

    out
}

/// Generate a diff for a new file (no previous content).
pub fn diff_for_create(path: &str, content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return format!("--- /dev/null\n+++ {path}\n@@ -0,0 +1,0 @@\n");
    }

    let mut out = String::new();
    out.push_str("--- /dev/null\n");
    out.push_str(&format!("+++ {path}\n"));
    out.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
    for line in &lines {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Generate a diff for a deleted file.
pub fn diff_for_delete(path: &str, content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return format!("--- {path}\n+++ /dev/null\n@@ -1,0 +0,0 @@\n");
    }

    let mut out = String::new();
    out.push_str(&format!("--- {path}\n"));
    out.push_str("+++ /dev/null\n");
    out.push_str(&format!("@@ -1,{} +0,0 @@\n", lines.len()));
    for line in &lines {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Internal diff engine (Myers-style longest-common-subsequence)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Edit {
    Keep(usize, usize), // (old_idx, new_idx)
    Delete(usize),      // old_idx
    Insert(usize),      // new_idx
}

/// Compute diff edits using a simple LCS-based approach.
fn diff_edits<'a>(old: &'a [&'a str], new: &'a [&'a str]) -> Vec<Edit> {
    let m = old.len();
    let n = new.len();

    // Build LCS table.
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            dp[i][j] = if old[i] == new[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    // Trace back to build edit list.
    let mut edits = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < m || j < n {
        if i < m && j < n && old[i] == new[j] {
            edits.push(Edit::Keep(i, j));
            i += 1;
            j += 1;
        } else if j < n && (i >= m || dp[i][j + 1] >= dp[i + 1][j]) {
            edits.push(Edit::Insert(j));
            j += 1;
        } else {
            edits.push(Edit::Delete(i));
            i += 1;
        }
    }
    edits
}

const CONTEXT: usize = 3;

/// Group edits into hunk strings.
fn compute_hunks(old: &[&str], new: &[&str]) -> Vec<String> {
    let edits = diff_edits(old, new);
    if edits.is_empty() {
        return Vec::new();
    }

    // Collect changed positions.
    let changed: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e, Edit::Keep(..)))
        .map(|(idx, _)| idx)
        .collect();

    if changed.is_empty() {
        return Vec::new();
    }

    // Group into hunk ranges with context.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let start = changed[0].saturating_sub(CONTEXT);
    let end = (changed[0] + CONTEXT + 1).min(edits.len());
    ranges.push((start, end));

    for &pos in &changed[1..] {
        let hunk_start = pos.saturating_sub(CONTEXT);
        let hunk_end = (pos + CONTEXT + 1).min(edits.len());
        let last = ranges.last_mut().unwrap();
        if hunk_start <= last.1 {
            last.1 = hunk_end.max(last.1);
        } else {
            ranges.push((hunk_start, hunk_end));
        }
    }

    let mut hunks = Vec::new();

    for (range_start, range_end) in ranges {
        let slice = &edits[range_start..range_end];

        // Count old and new lines for the hunk header.
        let (mut old_start, mut new_start) = (0usize, 0usize);
        let (mut old_count, mut new_count) = (0usize, 0usize);
        let mut first = true;

        for edit in slice {
            match edit {
                Edit::Keep(oi, ni) => {
                    if first {
                        old_start = *oi + 1;
                        new_start = *ni + 1;
                        first = false;
                    }
                    old_count += 1;
                    new_count += 1;
                }
                Edit::Delete(oi) => {
                    if first {
                        old_start = *oi + 1;
                        new_start = 0; // will be overridden by a Keep or Insert
                        first = false;
                    }
                    old_count += 1;
                }
                Edit::Insert(ni) => {
                    if first {
                        old_start = 0;
                        new_start = *ni + 1;
                        first = false;
                    }
                    new_count += 1;
                }
            }
        }

        let mut hunk = format!(
            "@@ -{},{} +{},{} @@\n",
            old_start, old_count, new_start, new_count
        );

        for edit in slice {
            match edit {
                Edit::Keep(oi, _) => {
                    hunk.push(' ');
                    hunk.push_str(old[*oi]);
                    hunk.push('\n');
                }
                Edit::Delete(oi) => {
                    hunk.push('-');
                    hunk.push_str(old[*oi]);
                    hunk.push('\n');
                }
                Edit::Insert(ni) => {
                    hunk.push('+');
                    hunk.push_str(new[*ni]);
                    hunk.push('\n');
                }
            }
        }

        hunks.push(hunk);
    }

    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_produces_empty_diff() {
        let d = unified_diff("f.txt", "hello\n", "hello\n");
        assert!(d.is_empty());
    }

    #[test]
    fn single_line_change_produces_diff() {
        let d = unified_diff("f.txt", "hello\n", "world\n");
        assert!(d.contains("---"));
        assert!(d.contains("+++"));
        assert!(d.contains("-hello"));
        assert!(d.contains("+world"));
    }

    #[test]
    fn create_diff_has_no_old_file() {
        let d = diff_for_create("new.rs", "fn main() {}\n");
        assert!(d.contains("--- /dev/null"));
        assert!(d.contains("+++ new.rs"));
        assert!(d.contains("+fn main() {}"));
    }

    #[test]
    fn delete_diff_has_no_new_file() {
        let d = diff_for_delete("old.rs", "fn main() {}\n");
        assert!(d.contains("+++ /dev/null"));
        assert!(d.contains("-fn main() {}"));
    }
}
