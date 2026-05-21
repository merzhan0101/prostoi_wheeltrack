use std::process::Command;

fn main() {
    // Время сборки (UTC)
    let build_time = chrono::Utc::now().to_rfc3339();

    // Git hash (короткий)
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Формируем строку вида: "2025-09-16T14:35:00Z (a1b2c3d)"
    let version = format!("{} ({})", build_time, git_hash);

    // Передаём в код
    println!("cargo:rustc-env=BUILD_VERSION={}", version);
}

// fn main() {
//     let timestamp = chrono::Utc::now().timestamp().to_string();
//     println!("cargo:rustc-env=BUILD_VERSION={}", timestamp);
// }
