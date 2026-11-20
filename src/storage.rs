//! Only the high score is persisted. Unreadable storage falls back to zero.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn rocketpult_load_best() -> u32;
    fn rocketpult_save_best(score: u32);
}

pub fn load() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        unsafe { rocketpult_load_best() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }
}

pub fn save(score: u32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        rocketpult_save_best(score);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, score.to_string());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    let root = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ".".into())
        .join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ".".into());
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| ".".into())
                .join(".local/share")
        });
    root.join("RocketpultRust/best-score.txt")
}
