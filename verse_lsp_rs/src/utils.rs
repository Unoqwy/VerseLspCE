use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ignore::{WalkBuilder, WalkState};

#[macro_export]
macro_rules! profile {
    ($name:expr, $block:expr $(;)?) => {
        if ::log::log_enabled!(log::Level::Debug) {
            let start_at = ::std::time::Instant::now();
            let result = { $block };
            log::debug!("[PROFILE] '{}' took: {:.3?}", $name, start_at.elapsed());
            result
        } else {
            $block
        }
    };
}

/// Traverses a path to collect all files with a given extension.
/// Uses parallel traversal.
pub fn collect_files_with_extension(path: &Path, file_extension: &str) -> Vec<PathBuf> {
    let result_paths = Arc::new(Mutex::new(Vec::new()));

    let walker = WalkBuilder::new(path)
        .standard_filters(false)
        .follow_links(true)
        .build_parallel();
    walker.run(|| {
        let result_paths = result_paths.clone();

        Box::new(move |result| {
            let Ok(dir_entry) = result else {
                return WalkState::Continue;
            };

            // speeds things up for big projects where __ExternalActors__ is massive
            if matches!(
                dir_entry.file_name().to_str(),
                Some(".git" | ".urc" | "__ExternalActors__" | "__ExternalObjects__")
            ) {
                return WalkState::Skip;
            }

            if let Some(extension) = dir_entry.path().extension()
                && extension.eq(file_extension)
            {
                let path_buf = dir_entry.path().to_path_buf();
                let mut acc = result_paths.lock().unwrap();
                acc.push(path_buf);
            }

            WalkState::Continue
        })
    });

    Arc::try_unwrap(result_paths)
        .expect("One strong reference")
        .into_inner()
        .unwrap_or_default()
}
