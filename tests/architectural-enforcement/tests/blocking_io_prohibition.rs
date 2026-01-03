//! Integration Test: Blocking I/O Prohibition
//!
//! This test enforces PRINCIPLE-efficiency.md Law 1 Part B: All I/O Must Be Async
//!
//! **Policy**: Production code in TUI and Conductor MUST NOT use blocking I/O.
//! **Required**: Use `tokio::fs`, `tokio::net`, `tokio::process`, not `std::fs`, `std::net`, `std::process`
//!
//! See: reference/PRINCIPLE-efficiency.md
//! See: reference/FORBIDDEN-inefficient-calculations.md (Category 6)

use std::fs;
use std::path::Path;

/// Test that production code does not use blocking I/O
#[test]
fn test_no_blocking_io_in_production_code() {
    let violations = find_blocking_io_violations();

    if !violations.is_empty() {
        eprintln!("\n❌ CRITICAL: Blocking I/O calls found in production code!");
        eprintln!("See reference/PRINCIPLE-efficiency.md Law 1 Part B: All I/O Must Be Async\n");

        for violation in &violations {
            eprintln!("  ❌ {}", violation);
        }

        eprintln!("\n❌ FORBIDDEN blocking I/O:");
        eprintln!("  - std::fs::read(), std::fs::write(), std::fs::File");
        eprintln!("  - std::net::TcpStream, std::net::TcpListener");
        eprintln!("  - std::io::Read, std::io::Write (blocking traits)");
        eprintln!("  - std::process::Command::output()");
        eprintln!("  - reqwest::blocking::*");
        eprintln!("\n✅ REQUIRED async I/O:");
        eprintln!("  - tokio::fs::read().await, tokio::fs::write().await");
        eprintln!("  - tokio::net::TcpStream::connect().await");
        eprintln!("  - tokio::io::AsyncRead, tokio::io::AsyncWrite");
        eprintln!("  - tokio::process::Command::output().await");
        eprintln!("  - reqwest::get().await");
        eprintln!("\n✅ ACCEPTABLE blocking I/O:");
        eprintln!("  - Non-async functions (before tokio runtime starts)");
        eprintln!("  - Test code");
        eprintln!("  - CLI argument parsing (before main runtime)");

        panic!(
            "\nFound {} blocking I/O violation(s) in production code.\nFix these before merging!",
            violations.len()
        );
    }
}

/// Find all blocking I/O calls in production code
fn find_blocking_io_violations() -> Vec<String> {
    let mut violations = Vec::new();

    // Check TUI production code
    check_directory("tui/src", &mut violations, &BlockingIoPolicy::default());

    // Check Conductor production code
    check_directory(
        "conductor/core/src",
        &mut violations,
        &BlockingIoPolicy::default(),
    );

    check_directory(
        "conductor/daemon/src",
        &mut violations,
        &BlockingIoPolicy::default(),
    );

    violations
}

#[derive(Default)]
struct BlockingIoPolicy {
    // Policy flags could go here if needed
}

fn check_directory(dir: &str, violations: &mut Vec<String>, policy: &BlockingIoPolicy) {
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

fn check_file(path: &Path, violations: &mut Vec<String>, _policy: &BlockingIoPolicy) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Skip comments
        let code_part = line.split("//").next().unwrap_or(line);

        // Skip if in test function
        if is_in_test_function(&lines, idx) {
            continue;
        }

        // Skip if in non-async function (before runtime)
        if is_in_non_async_function(&lines, idx) {
            continue;
        }

        // Check for blocking file system I/O
        if code_part.contains("std::fs::")
            || (code_part.contains("use std::fs") && !code_part.contains("//"))
        {
            violations.push(format!(
                "{}:{} - Blocking file I/O: {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }

        // Check for blocking network I/O
        if code_part.contains("std::net::")
            || (code_part.contains("use std::net") && !code_part.contains("//"))
        {
            violations.push(format!(
                "{}:{} - Blocking network I/O: {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }

        // Check for blocking process I/O
        if code_part.contains("std::process::Command") && !code_part.contains("tokio::process") {
            violations.push(format!(
                "{}:{} - Blocking process I/O: {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }

        // Check for blocking HTTP client
        if code_part.contains("reqwest::blocking") {
            violations.push(format!(
                "{}:{} - Blocking HTTP client: {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }

        // Check for blocking stdin/stdout (in async context)
        if (code_part.contains("std::io::stdin()") || code_part.contains("std::io::stdout()"))
            && is_in_async_function(&lines, idx)
        {
            violations.push(format!(
                "{}:{} - Blocking stdin/stdout in async: {}",
                path.display(),
                line_number,
                line.trim()
            ));
        }
    }
}

/// Check if line is inside a test function
fn is_in_test_function(lines: &[&str], current_idx: usize) -> bool {
    // Scan backwards to find the enclosing function
    let mut found_fn_idx = None;
    for i in (0..current_idx).rev() {
        let line = lines[i].trim();

        if line.starts_with("fn ") || line.contains(" fn ") {
            found_fn_idx = Some(i);
            break;
        }

        // Stop at module boundaries
        if line.starts_with("mod ") || (line.starts_with("impl ") && line.contains('{')) {
            return false;
        }
    }

    // If we found a function, check if it has a test marker
    if let Some(fn_idx) = found_fn_idx {
        // Scan backwards from the function to find test markers
        for i in (0..fn_idx).rev() {
            let line = lines[i].trim();

            if line.starts_with("#[test]")
                || line.starts_with("#[tokio::test")
                || line.starts_with("#[cfg(test)]")
            {
                return true;
            }

            // Stop if we hit another function or boundary
            if line.starts_with("fn ") || line.starts_with("mod ") || line.starts_with("impl ") {
                break;
            }
        }
    }

    false
}

/// Check if line is inside an async function
fn is_in_async_function(lines: &[&str], current_idx: usize) -> bool {
    // Scan backwards for async fn
    for i in (0..current_idx).rev() {
        let line = lines[i].trim();

        if line.contains("async fn ") {
            return true;
        }

        if line.starts_with("fn ") && !line.contains("async") {
            return false; // Found non-async function
        }

        // Stop at module/impl boundaries
        if line.starts_with("mod ") || (line.starts_with("impl ") && line.contains('{')) {
            return false;
        }
    }
    false
}

/// Check if line is inside a non-async function (acceptable for blocking I/O)
fn is_in_non_async_function(lines: &[&str], current_idx: usize) -> bool {
    // Scan backwards for fn (without async)
    for i in (0..current_idx).rev() {
        let line = lines[i].trim();

        if line.starts_with("fn ") && !line.contains("async") {
            // Non-async function - blocking I/O is OK here
            return true;
        }

        if line.contains("async fn ") {
            return false; // Found async function
        }

        // Stop at module/impl boundaries
        if line.starts_with("mod ") || (line.starts_with("impl ") && line.contains('{')) {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_io_detection() {
        // This test verifies that the detector itself works
        let test_code = vec![
            "async fn bad_function() {",
            "    let contents = std::fs::read_to_string(\"file.txt\")?;",
            "}",
        ];

        // Should detect this is in async function
        assert!(
            is_in_async_function(&test_code, 1),
            "Should detect async function"
        );

        // Should not be in non-async function
        assert!(
            !is_in_non_async_function(&test_code, 1),
            "Should not be in non-async function"
        );
    }

    #[test]
    fn test_non_async_function_detection() {
        let test_code = vec![
            "fn main() {",
            "    let contents = std::fs::read_to_string(\"config.toml\")?;",
            "}",
        ];

        // Should detect non-async function (acceptable)
        assert!(
            is_in_non_async_function(&test_code, 1),
            "Should detect non-async function"
        );
    }

    #[test]
    fn test_test_function_detection() {
        let test_code = vec![
            "#[test]",
            "fn test_something() {",
            "    let contents = std::fs::read_to_string(\"test.txt\")?;",
            "}",
        ];

        // Should detect test function (acceptable)
        assert!(
            is_in_test_function(&test_code, 2),
            "Should detect test function"
        );
    }
}
