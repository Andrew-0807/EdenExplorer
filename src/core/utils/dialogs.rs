use crate::core::utils::files::{OpRecord, recording_sink};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::UI::Shell::{
    FILEOPERATION_FLAGS, FileOperation, IFileOperation, IShellItem, SHCreateItemFromParsingName,
};
use windows::core::{PCWSTR, Result};

fn same_drive(a: &Path, b: &Path) -> bool {
    a.components().next() == b.components().next()
}

/// Moves (same drive) or copies (other drive) `sources` into `destination`.
/// Returns what the shell reports as done, for undo.
pub fn show_copy_move_dialog(sources: Vec<PathBuf>, destination: &Path) -> Result<OpRecord> {
    if sources.is_empty() {
        return Ok(OpRecord::default());
    }

    unsafe {
        (|| {
            let file_op: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;

            // Minimal flags (safe default)
            file_op.SetOperationFlags(FILEOPERATION_FLAGS(0))?;
            let (sink, recorder) = recording_sink();
            let cookie = file_op.Advise(&sink)?;

            // Destination
            let dest_w: Vec<u16> = destination
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();

            let dest_item: IShellItem = SHCreateItemFromParsingName(PCWSTR(dest_w.as_ptr()), None)?;

            for src in &sources {
                let src_w: Vec<u16> = src.as_os_str().encode_wide().chain(Some(0)).collect();

                let src_item: IShellItem =
                    SHCreateItemFromParsingName(PCWSTR(src_w.as_ptr()), None)?;

                if same_drive(src, destination) {
                    // 4th argument is the rename-to item; keep None for same-name move/copy.
                    file_op.MoveItem(&src_item, Some(&dest_item), None, None)?;
                } else {
                    file_op.CopyItem(&src_item, Some(&dest_item), None, None)?;
                }
            }

            let performed = file_op.PerformOperations();
            let _ = file_op.Unadvise(cookie);
            performed?;

            let mut record = recorder.take();
            record.retain_sources(&sources);
            Ok(record)
        })()
    }
}
