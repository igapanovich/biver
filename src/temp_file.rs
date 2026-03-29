use std::env;
use std::env::temp_dir;
use std::path::PathBuf;
use uuid::Uuid;

pub fn path() -> PathBuf {
    let file_name = Uuid::new_v4().to_string();

    let temp_dir = if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
    } else {
        temp_dir()
    };

    temp_dir.join("biver").join(file_name)
}
