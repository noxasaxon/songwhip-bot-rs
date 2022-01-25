use serde::Serialize;
use serde_json::to_writer_pretty;
use std::fs::File;

/// used during development to capture/inspect for mocking
pub fn write_serde_struct_to_file(file_name: &str, obj: impl Serialize) {
    to_writer_pretty(
        &File::create(file_name).expect("unable to create file"),
        &obj,
    )
    .expect("unable to write to file")
}
