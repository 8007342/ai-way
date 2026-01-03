//! Integration Test: Sleep Prohibition
//!
//! This test enforces PRINCIPLE-efficiency.md Law 1: No Sleep, Only Wait on I/O
//!
//! **Policy**: Production code in TUI and Conductor MUST NOT call sleep methods.
//! **Exceptions**: Frame rate limiting (TUI only), exponential backoff (retry logic only), test code
//!
//! See: reference/PRINCIPLE-efficiency.md
//! See: reference/FORBIDDEN-inefficient-calculations.md

use std::fs;
use std::path::Path;

/// Test that production code does not contain sleep() calls
#[test]
fn test_no_sleep_in_production_code() {
    let violations = find_sleep_violations();

    if !violations.is_empty() {
        eprintln!("\n❌ CRITICAL: Sleep calls found in production code!");
        eprintln!("See reference/PRINCIPLE-efficiency.md Law 1: No Sleep\n");

        for violation in &violations {
            eprintln!("  ❌ {}", violation);
        }

        eprintln!("\n✅ ACCEPTABLE sleep uses:");
        eprintln!("  - Frame rate limiting in TUI (tokio::time::sleep in frame control)");
        eprintln!("  - Exponential backoff in retry logic");
        eprintln!("  - Test code (#[test] or #[tokio::test] functions)");
        eprintln!("  - Periodic tasks using tokio::time::interval()");
        eprintln!("\n❌ FORBIDDEN:");
        eprintln!("  - Sleep in polling loops");
        eprintln!("  - Sleep as poor man's synchronization");
        eprintln!("  - Sleep to 'wait' for events (use async I/O!)");

        panic!(
            "\nFound {} sleep violation(s) in production code.\nFix these before merging!",
            violations.len()
        );
    }
}

/// Find all sleep() calls in production code
fn find_sleep_violations() -> Vec<String> {
    let mut violations = Vec::new();

    // Check TUI production code
    check_directory(
        "tui/src",
        &mut violations,
        &SleepPolicy {
            allow_frame_limiting: true,
            allow_backoff: true,
            allow_tests: false,
        },
    );

    // Check Conductor production code
    check_directory(
        "conductor/core/src",
        &mut violations,
        &SleepPolicy {
            allow_frame_limiting: false,
            allow_backoff: true,
            allow_tests: false,
        },
    );

    check_directory(
        "conductor/daemon/src",
        &mut violations,
        &SleepPolicy {
            allow_frame_limiting: false,
            allow_backoff: true,
            allow_tests: false,
        },
    );

    violations
}

struct SleepPolicy {
    allow_frame_limiting: bool,
    allow_backoff: bool,
    allow_tests: bool,
}

fn check_directory(dir: &str, violations: &mut Vec<String>, policy: &SleepPolicy) {
    let path = Path::new(dir);
    if !path.exists() {
        return;
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("rs") {
            check_file(entry.path(), violations, policy);
        }
    }
}

fn check_file(path: &Path, violations: &mut Vec<String>, policy: &SleepPolicy) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Skip comments
        let code_part = line.split("//").next().unwrap_or(line);

        // Check for sleep calls
        if code_part.contains("::sleep(") || code_part.contains(".sleep(") {
            // Check if it's in a test function
            if policy.allow_tests && is_in_test_function(&lines, idx) {
                continue;
            }

            // Check if it's frame limiting (only in TUI app.rs)
            if policy.allow_frame_limiting
                && path.ends_with("tui/src/app.rs")
                && is_frame_limiting_context(&lines, idx)
            {
                continue;
            }

            // Check if it's exponential backoff
            if policy.allow_backoff && is_backoff_context(&lines, idx) {
                continue;
            }

            // Check if it's using tokio::time::interval (acceptable)
            if is_interval_pattern(&lines, idx) {
                continue;
            }

            violations.push(format!(
                "{}:{} - {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }
    }
}

/// Check if line is inside a test function
fn is_in_test_function(lines: &[&str], current_idx: usize) -> bool {
    // Scan backwards for #[test] or #[tokio::test]
    for i in (0..current_idx).rev() {
        let line = lines[i].trim();

        if line.starts_with("fn ") && !line.contains("test") {
            return false; // Found a non-test function first
        }

        if line.starts_with("#[test]") || line.starts_with("#[tokio::test") {
            return true;
        }

        // Stop at module boundaries
        if line.starts_with("mod ") || line.starts_with("impl ") {
            return false;
        }
    }
    false
}

/// Check if sleep is used for frame rate limiting (acceptable in TUI)
fn is_frame_limiting_context(lines: &[&str], current_idx: usize) -> bool {
    // Look for frame_duration, frame rate, or FPS in nearby lines
    let context_range = current_idx.saturating_sub(10)..std::cmp::min(current_idx + 5, lines.len());

    for i in context_range {
        let line = lines[i].to_lowercase();
        if line.contains("frame")
            || line.contains("fps")
            || line.contains("rate limit")
            || line.contains("tick_rate")
        {
            return true;
        }
    }
    false
}

/// Check if sleep is used for exponential backoff (acceptable for retry logic)
fn is_backoff_context(lines: &[&str], current_idx: usize) -> bool {
    // Look for backoff, retry, reconnect in nearby lines
    let context_range = current_idx.saturating_sub(15)..std::cmp::min(current_idx + 5, lines.len());

    let mut has_backoff_calc = false;
    let mut has_retry_context = false;

    for i in context_range {
        let line = lines[i].to_lowercase();

        // Check for exponential backoff calculation (2^n pattern or bit shift)
        if line.contains("<<") || line.contains("pow") || line.contains("* 2") {
            has_backoff_calc = true;
        }

        // Check for retry/reconnect context
        if line.contains("retry")
            || line.contains("reconnect")
            || line.contains("backoff")
            || line.contains("attempt")
        {
            has_retry_context = true;
        }
    }

    has_backoff_calc && has_retry_context
}

/// Check if this is tokio::time::interval pattern (acceptable for periodic tasks)
fn is_interval_pattern(lines: &[&str], current_idx: usize) -> bool {
    // Check if we're inside a loop that uses interval.tick()
    // This is acceptable: let mut interval = tokio::time::interval(...); loop { interval.tick().await; }

    // Look backwards for interval usage
    let context_range = current_idx.saturating_sub(20)..current_idx;

    for i in context_range {
        let line = lines[i];
        if line.contains("interval.tick()") || line.contains("tokio::time::interval") {
            return true;
        }
    }

    // Also check forward a bit
    let forward_range = current_idx..std::cmp::min(current_idx + 5, lines.len());
    for i in forward_range {
        let line = lines[i];
        if line.contains("interval.tick()") {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_violation_detection() {
        // This test verifies that the detector itself works
        let test_code = vec![
            "fn bad_function() {",
            "    tokio::time::sleep(Duration::from_millis(10)).await;",
            "}",
        ];

        let policy = SleepPolicy {
            allow_frame_limiting: false,
            allow_backoff: false,
            allow_tests: false,
        };

        // Should detect the sleep
        assert!(
            !is_in_test_function(&test_code, 1),
            "Should detect this is not a test"
        );
    }

    #[test]
    fn test_backoff_detection() {
        let test_code = vec![
            "fn reconnect() {",
            "    let delay = base_delay * (1 << attempt);",
            "    println!(\"Retry attempt {}\", attempt);",
            "    tokio::time::sleep(Duration::from_millis(delay)).await;",
            "}",
        ];

        assert!(
            is_backoff_context(&test_code, 3),
            "Should detect exponential backoff pattern"
        );
    }

    #[test]
    fn test_frame_limiting_detection() {
        let test_code = vec![
            "fn render_loop() {",
            "    let frame_duration = Duration::from_millis(100); // 10 FPS",
            "    loop {",
            "        render();",
            "        tokio::time::sleep(frame_duration).await;",
            "    }",
            "}",
        ];

        assert!(
            is_frame_limiting_context(&test_code, 4),
            "Should detect frame rate limiting"
        );
    }
}
