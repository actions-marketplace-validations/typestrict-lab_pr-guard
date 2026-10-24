//! Source language by file extension. Files that are not code (docs, config, data) have none and are never "not evaluated".

use std::path::Path;

/// The languages TS-PRGuard is trained and evaluated on. iOS is Swift only: Objective-C is "not evaluated".
pub const SUPPORTED: &[&str] = &["javascript", "typescript", "go", "rust", "python", "dart", "kotlin", "java", "swift", "php", "ruby"];

pub fn of(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => "javascript",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "go" => "go",
        "rs" => "rust",
        "py" | "pyi" => "python",
        "dart" => "dart",
        "kt" | "kts" => "kotlin",
        "java" => "java",
        "swift" => "swift",
        "m" | "mm" => "objective-c",
        "php" => "php",
        "rb" | "rake" | "gemspec" => "ruby",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "c++",
        "cs" => "c#",
        "scala" => "scala",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "lua" => "lua",
        "pl" | "pm" => "perl",
        "r" => "r",
        "clj" | "cljs" => "clojure",
        "fs" | "fsx" => "f#",
        "zig" => "zig",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" => "powershell",
        // SQL is not here: the destructive_sql rule reads migrations, the model does not.
        _ => return None,
    })
}
