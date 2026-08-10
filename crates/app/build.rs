fn main() {
    // The tauri context macro requires the frontend dist dir to exist even on
    // a fresh clone that has never built the ui.
    if let Err(e) = std::fs::create_dir_all("../../ui/dist") {
        panic!("cannot create ui/dist: {e}");
    }
    tauri_build::build()
}
