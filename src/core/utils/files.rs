use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::{copy, create_dir_all, read_dir};
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use windows::Win32::Foundation::HWND;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_EnumItems, BHID_SFUIObject, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX,
    FOLDERID_RecycleBinFolder, IContextMenu, IEnumShellItems, IFileOperationProgressSink,
    IFileOperationProgressSink_Impl, ILFree, ILGetSize, IShellItem, IShellLinkW,
    SHCreateItemFromIDList, SHFILEINFOW, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW,
    SHGetIDListFromObject, SHGetKnownFolderIDList, SIGDN_FILESYSPATH, SLGP_RAWPATH,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{Error, HRESULT, Interface, PCSTR, PCWSTR, PWSTR, Ref, implement};

/// Resolve a Windows shortcut to the filesystem path it targets.
pub fn resolve_shortcut_target(path: &Path) -> Option<PathBuf> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        return None;
    }

    let shortcut_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(
            &windows::core::GUID::from_u128(0x00021401_0000_0000_c000_000000000046),
            None,
            CLSCTX_INPROC_SERVER,
        )
        .ok()?;
        let persist_file: IPersistFile = shell_link.cast().ok()?;
        persist_file
            .Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READ)
            .ok()?;

        let mut target = vec![0u16; 32_768];
        let mut find_data = WIN32_FIND_DATAW::default();
        shell_link
            .GetPath(&mut target, &mut find_data, SLGP_RAWPATH.0 as u32)
            .ok()?;

        let target_len = target.iter().position(|&value| value == 0)?;
        if target_len == 0 {
            return None;
        }

        let target = PathBuf::from(OsString::from_wide(&target[..target_len]));
        Some(if target.is_relative() {
            path.parent().unwrap_or_else(|| Path::new(".")).join(target)
        } else {
            target
        })
    }
}

pub fn get_file_type_name<'a>(ext: &str, cache: &'a mut HashMap<String, String>) -> &'a str {
    use std::collections::hash_map::Entry;

    match cache.entry(ext.to_string()) {
        Entry::Occupied(entry) => entry.into_mut().as_str(),
        Entry::Vacant(entry) => {
            // Ensure extension starts with "."
            let ext_formatted = if ext.starts_with('.') {
                ext.to_string()
            } else {
                format!(".{}", ext)
            };

            let wide: Vec<u16> = OsStr::new(&ext_formatted)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut info = SHFILEINFOW::default();

            let _result = unsafe {
                SHGetFileInfoW(
                    PCWSTR(wide.as_ptr()),
                    FILE_ATTRIBUTE_NORMAL,
                    Some(&mut info),
                    size_of::<SHFILEINFOW>() as u32,
                    SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
                )
            };

            let len = info.szTypeName.iter().position(|&c| c == 0).unwrap_or(0);
            let type_name = String::from_utf16_lossy(&info.szTypeName[..len]);

            entry.insert(type_name).as_str()
        }
    }
}

pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size_f = size as f64;
    let mut unit_index = 0;

    while size_f >= 1024.0 && unit_index < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_index])
    }
}

#[allow(dead_code)]
pub fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    create_dir_all(dest)?;

    for entry in read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let new_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &new_path)?;
        } else {
            copy(entry.path(), new_path)?;
        }
    }

    Ok(())
}

/// Checks if a filename contains valid characters for real-time validation
/// Used during typing to immediately filter invalid characters
pub fn filename_has_valid_characters_realtime(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check maximum length
    if name.len() > 255 {
        return false;
    }

    // Windows reserved characters that cannot be used in filenames
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    // Check for invalid characters
    for ch in name.chars() {
        if invalid_chars.contains(&ch) {
            return false;
        }
    }

    // Windows reserved names (case-insensitive)
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let name_upper = name.to_uppercase();
    for reserved in &reserved_names {
        if name_upper == *reserved {
            return false;
        }
    }

    true
}

/// Progress messages streamed from a background shell file-operation worker to
/// the UI thread.
#[derive(Clone, Debug)]
pub enum FileOpProgress {
    /// The shell operation began (fires after any confirmation prompt is accepted).
    Started,
    /// Overall work counters reported by the shell.
    Progress { total: u32, done: u32 },
    /// Absolute path of the item currently being processed.
    Item { name: String },
    /// The operation finished. `cancelled` reflects a user-initiated abort.
    Finished {
        cancelled: bool,
        error: Option<String>,
    },
}

/// Which shell operation `file_op_async` should perform on the given sources.
pub enum FileOpKind {
    /// Delete to the recycle bin (`allow_undo`) or permanently.
    Delete { allow_undo: bool },
    /// Move every source into `dest`.
    Move { dest: PathBuf },
    /// Copy every source into `dest`.
    Copy { dest: PathBuf },
}

/// What a shell operation did, as reported by its `Post*Item` callbacks. The
/// shell may pick a different name than requested (collisions), so these are
/// the final paths. Includes only items whose callback reported success.
#[derive(Default)]
pub struct OpRecord {
    /// `(old, new)`.
    pub moved: Vec<(PathBuf, PathBuf)>,
    /// `(source, new)`.
    pub copied: Vec<(PathBuf, PathBuf)>,
    /// Originals the shell sent to the Recycle Bin (it reported a Bin item, a
    /// permanent delete reports none). The Bin item's own PIDL is not usable
    /// for restoring, so entries are looked up with `find_recycle_bin_pidls`.
    pub trashed: Vec<PathBuf>,
    /// `(old, new)`.
    pub renamed: Vec<(PathBuf, PathBuf)>,
    /// First failing HRESULT among the `Post*Item` callbacks.
    pub failure: Option<HRESULT>,
}

/// Shell paths compare case-insensitively and may differ in slash style.
fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

impl OpRecord {
    /// Drops entries for items the shell reported below the ones we asked for
    /// (children of a folder), keeping one entry per requested source.
    pub fn retain_sources(&mut self, sources: &[PathBuf]) {
        let keys: HashSet<String> = sources.iter().map(|p| path_key(p)).collect();
        let wanted = |p: &Path| keys.contains(&path_key(p));
        self.moved.retain(|(old, _)| wanted(old));
        self.copied.retain(|(src, _)| wanted(src));
        self.trashed.retain(|original| wanted(original));
        self.renamed.retain(|(old, _)| wanted(old));
    }
}

/// Collects an [`OpRecord`] from a progress sink. Callbacks run on the thread
/// that calls `PerformOperations`, the caller reads the result afterwards.
#[derive(Default)]
pub struct OpRecorder {
    state: Mutex<(Vec<PathBuf>, OpRecord)>,
}

/// Files one finished item into the record: source, new path, and whether the
/// shell reported a new item at all.
type RecordStore = fn(&mut OpRecord, PathBuf, Option<PathBuf>, bool);

impl OpRecorder {
    /// Takes the record, leaving the recorder empty.
    pub fn take(&self) -> OpRecord {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.0.clear();
        std::mem::take(&mut guard.1)
    }

    /// `Pre*Item` runs while the item still exists. Pre and Post nest like a
    /// stack, so the source path is read here and popped by the matching Post.
    fn pre(&self, source: Option<String>) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.0.push(source.map(PathBuf::from).unwrap_or_default());
    }

    fn post(&self, hr: HRESULT, newly_created: Ref<'_, IShellItem>, store: RecordStore) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let source = guard.0.pop().unwrap_or_default();
        let record = &mut guard.1;
        if hr.is_err() {
            record.failure.get_or_insert(hr);
            return;
        }
        if source.as_os_str().is_empty() {
            return;
        }
        let item = newly_created.ok().ok();
        let new_path = item.and_then(item_path).map(PathBuf::from);
        store(record, source, new_path, item.is_some());
    }
}

fn item_path(item: &IShellItem) -> Option<String> {
    let pwstr = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    let text = unsafe { pwstr_to_string(pwstr) };
    unsafe { CoTaskMemFree(Some(pwstr.0 as _)) };
    if text.is_empty() { None } else { Some(text) }
}

/// Absolute PIDL bytes of a shell item.
fn pidl_bytes_of(item: &IShellItem) -> Option<Vec<u8>> {
    unsafe {
        let pidl = SHGetIDListFromObject(item).ok()?;
        let size = ILGetSize(Some(pidl as _)) as usize;
        let mut bytes = vec![0u8; size];
        std::ptr::copy_nonoverlapping(pidl as *const u8, bytes.as_mut_ptr(), size);
        ILFree(Some(pidl as _));
        Some(bytes)
    }
}

/// Finds the Recycle Bin entries of files deleted from `originals`. With several
/// matches the newest deletion wins. One Bin enumeration serves all of them.
/// Blocks on the shell. The result is parallel to `originals`.
pub fn find_recycle_bin_pidls(originals: &[PathBuf]) -> Vec<Option<Vec<u8>>> {
    use crate::core::fs::{DateStyle, get_shell_item_metadata};

    // (folder, lowercase name) -> index. The Bin's display name omits the
    // extension when Explorer hides it, so the stem is a key as well.
    let mut wanted: HashMap<(String, String), usize> = HashMap::new();
    for (index, original) in originals.iter().enumerate() {
        let Some(parent) = original.parent() else {
            continue;
        };
        let dir = path_key(parent);
        for name in [original.file_name(), original.file_stem()]
            .into_iter()
            .flatten()
        {
            wanted.insert((dir.clone(), name.to_string_lossy().to_lowercase()), index);
        }
    }

    let mut best: Vec<Option<(i64, Vec<u8>)>> = vec![None; originals.len()];
    unsafe {
        let Ok(bin_pidl) = SHGetKnownFolderIDList(&FOLDERID_RecycleBinFolder, 0, None) else {
            return vec![None; originals.len()];
        };
        let bin = SHCreateItemFromIDList::<IShellItem>(bin_pidl);
        CoTaskMemFree(Some(bin_pidl as _));
        let Some(enum_items) = bin.ok().and_then(|bin| {
            bin.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems)
                .ok()
        }) else {
            return vec![None; originals.len()];
        };

        loop {
            let mut fetched: [Option<IShellItem>; 1] = [None];
            if enum_items.Next(&mut fetched, None).is_err() {
                break;
            }
            let Some(item) = fetched[0].take() else { break };

            let metadata = get_shell_item_metadata(&item, DateStyle::default(), true);
            let (deleted_raw, name, directory) = (metadata.6, metadata.7, metadata.8);
            let (Some(name), Some(directory)) = (name, directory) else {
                continue;
            };
            let key = (path_key(Path::new(&directory)), name.to_lowercase());
            let Some(&index) = wanted.get(&key) else {
                continue;
            };
            let deleted = deleted_raw.unwrap_or(0);
            if best[index]
                .as_ref()
                .is_some_and(|(newest, _)| *newest >= deleted)
            {
                continue;
            }
            if let Some(bytes) = pidl_bytes_of(&item) {
                best[index] = Some((deleted, bytes));
            }
        }
    }
    best.into_iter()
        .map(|entry| entry.map(|(_, bytes)| bytes))
        .collect()
}

/// A progress sink that only records results, for the blocking shell
/// operations that have no progress UI.
pub fn recording_sink() -> (IFileOperationProgressSink, Arc<OpRecorder>) {
    let recorder = Arc::new(OpRecorder::default());
    // The receiver is dropped on purpose: progress messages are discarded.
    let (tx, _rx) = crossbeam_channel::unbounded();
    let sink: IFileOperationProgressSink = FileOpProgressSink {
        tx,
        cancel: Arc::new(AtomicBool::new(false)),
        notify: Arc::new(|| {}),
        recorder: recorder.clone(),
        base_total: 0,
        pre_count: AtomicU32::new(0),
        post_count: AtomicU32::new(0),
    }
    .into();
    (sink, recorder)
}

#[implement(IFileOperationProgressSink)]
struct FileOpProgressSink {
    recorder: Arc<OpRecorder>,
    tx: Sender<FileOpProgress>,
    cancel: Arc<AtomicBool>,
    notify: Arc<dyn Fn() + Send + Sync>,
    /// Number of top-level items we asked the shell to process.
    base_total: u32,
    /// Items announced by a `Pre*Item` callback. This can grow for recursive
    /// operations where the shell discovers children after the operation starts.
    pre_count: AtomicU32,
    /// Items confirmed done via a `Post*Item` callback — the reliable per-item signal.
    post_count: AtomicU32,
}

impl FileOpProgressSink {
    fn emit(&self, msg: FileOpProgress) {
        let _ = self.tx.send(msg);
        (self.notify)();
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Report item counts from callbacks rather than the shell's
    /// `UpdateProgress` figures. Those figures represent shell work units and
    /// can make the UI reach X/X while callbacks are still arriving.
    fn emit_progress(&self) {
        let total = self.base_total.max(self.pre_count.load(Ordering::Relaxed));
        let done = self.post_count.load(Ordering::Relaxed).min(total);
        self.emit(FileOpProgress::Progress { total, done });
    }

    /// Shared `Pre*Item` body: abort if cancelled, else report the current item.
    fn report_item(&self, psiitem: Ref<'_, IShellItem>) -> windows::core::Result<()> {
        if self.cancelled() {
            return cancel_abort();
        }
        self.pre_count.fetch_add(1, Ordering::Relaxed);
        let name = shell_item_path(psiitem);
        self.recorder.pre(name.clone());
        if let Some(name) = name {
            self.emit(FileOpProgress::Item { name });
        }
        Ok(())
    }

    /// Shared `Post*Item` body: abort if cancelled, else advance the counter.
    fn report_post(&self) -> windows::core::Result<()> {
        if self.cancelled() {
            return cancel_abort();
        }
        self.post_count.fetch_add(1, Ordering::Relaxed);
        self.emit_progress();
        Ok(())
    }
}

/// A failed HRESULT from a progress-sink callback aborts the whole shell operation,
/// which is how the Cancel button stops an in-flight delete.
fn cancel_abort() -> windows::core::Result<()> {
    // HRESULT_FROM_WIN32(ERROR_CANCELLED)
    Err(Error::from(HRESULT(0x8007_04C7u32 as i32)))
}

fn shell_item_path(psiitem: Ref<'_, IShellItem>) -> Option<String> {
    let item = psiitem.ok().ok()?;
    let pwstr = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    let text = unsafe { pwstr_to_string(pwstr) };
    unsafe { CoTaskMemFree(Some(pwstr.0 as _)) };
    if text.is_empty() { None } else { Some(text) }
}

unsafe fn pwstr_to_string(pw: PWSTR) -> String {
    if pw.0.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    let mut ptr = pw.0;
    unsafe {
        while !ptr.is_null() && *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
        }
    }
    let slice = unsafe { std::slice::from_raw_parts(pw.0, len) };
    String::from_utf16_lossy(slice)
}

impl IFileOperationProgressSink_Impl for FileOpProgressSink_Impl {
    fn StartOperations(&self) -> windows::core::Result<()> {
        self.emit(FileOpProgress::Started);
        // Seed the total so the bar shows "0 / N" immediately instead of a spinner.
        self.emit_progress();
        Ok(())
    }

    fn FinishOperations(&self, _hrresult: HRESULT) -> windows::core::Result<()> {
        Ok(())
    }

    fn PreRenameItem(
        &self,
        _dwflags: u32,
        psiitem: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
    ) -> windows::core::Result<()> {
        self.recorder.pre(shell_item_path(psiitem));
        Ok(())
    }

    fn PostRenameItem(
        &self,
        _dwflags: u32,
        _psiitem: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
        hrrename: HRESULT,
        psinewlycreated: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.recorder
            .post(hrrename, psinewlycreated, |record, old, new, _| {
                if let Some(new) = new {
                    record.renamed.push((old, new));
                }
            });
        Ok(())
    }

    fn PreMoveItem(
        &self,
        _dwflags: u32,
        psiitem: Ref<'_, IShellItem>,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
    ) -> windows::core::Result<()> {
        self.report_item(psiitem)
    }

    fn PostMoveItem(
        &self,
        _dwflags: u32,
        _psiitem: Ref<'_, IShellItem>,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
        hrmove: HRESULT,
        psinewlycreated: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.recorder
            .post(hrmove, psinewlycreated, |record, old, new, _| {
                if let Some(new) = new {
                    record.moved.push((old, new));
                }
            });
        self.report_post()
    }

    fn PreCopyItem(
        &self,
        _dwflags: u32,
        psiitem: Ref<'_, IShellItem>,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
    ) -> windows::core::Result<()> {
        self.report_item(psiitem)
    }

    fn PostCopyItem(
        &self,
        _dwflags: u32,
        _psiitem: Ref<'_, IShellItem>,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
        hrcopy: HRESULT,
        psinewlycreated: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.recorder
            .post(hrcopy, psinewlycreated, |record, src, new, _| {
                if let Some(new) = new {
                    record.copied.push((src, new));
                }
            });
        self.report_post()
    }

    fn PreDeleteItem(
        &self,
        _dwflags: u32,
        psiitem: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.report_item(psiitem)
    }

    fn PostDeleteItem(
        &self,
        _dwflags: u32,
        _psiitem: Ref<'_, IShellItem>,
        hrdelete: HRESULT,
        psinewlycreated: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        // For a Recycle Bin delete the new item is the Bin entry (null when the
        // delete is permanent).
        self.recorder
            .post(hrdelete, psinewlycreated, |record, original, _, in_bin| {
                if in_bin {
                    record.trashed.push(original);
                }
            });
        self.report_post()
    }

    fn PreNewItem(
        &self,
        _dwflags: u32,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn PostNewItem(
        &self,
        _dwflags: u32,
        _psidestinationfolder: Ref<'_, IShellItem>,
        _psznewname: &PCWSTR,
        _psztemplatename: &PCWSTR,
        _dwfileattributes: u32,
        _hrnew: HRESULT,
        _psinewitem: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn UpdateProgress(&self, _iworktotal: u32, _iworksofar: u32) -> windows::core::Result<()> {
        if self.cancelled() {
            return cancel_abort();
        }
        self.emit_progress();
        Ok(())
    }

    fn ResetTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }

    fn PauseTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }

    fn ResumeTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Runs a shell file operation (delete/move/copy) on a background STA thread so
/// the UI thread never blocks (blocking it previously made Windows flag
/// EdenExplorer as "Not Responding" on large operations). Progress is streamed to
/// `tx`; `notify` requests a UI repaint from the worker so updates appear without
/// busy-polling.
///
/// Windows' own progress dialog is suppressed (`FOF_SILENT`) in favor of the
/// caller's UI, while confirmation prompts (recycle-bin / merge collisions) are
/// preserved (we do not set `FOF_NOCONFIRMATION`). `owner_hwnd` is the raw pointer
/// value of the window that should own any shell dialog (0 for none). Setting
/// `cancel` aborts the operation at the next item boundary.
pub fn file_op_async(
    kind: FileOpKind,
    sources: Vec<PathBuf>,
    owner_hwnd: isize,
    tx: Sender<FileOpProgress>,
    cancel: Arc<AtomicBool>,
    notify: Arc<dyn Fn() + Send + Sync>,
) -> Arc<OpRecorder> {
    let recorder = Arc::new(OpRecorder::default());
    if sources.is_empty() {
        let _ = tx.send(FileOpProgress::Finished {
            cancelled: false,
            error: None,
        });
        notify();
        return recorder;
    }

    let worker_recorder = recorder.clone();
    thread::spawn(move || {
        use windows::Win32::System::Com::{
            CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        };
        use windows::Win32::UI::Shell::{
            FILEOPERATION_FLAGS, FOF_ALLOWUNDO, FOF_SILENT, FileOperation, IFileOperation,
            SHCreateItemFromParsingName,
        };
        use windows::core::HSTRING;

        unsafe {
            // Shell operations and their dialogs need a single-threaded apartment
            // with a message pump, which PerformOperations provides on this thread.
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            let result = (|| -> windows::core::Result<()> {
                let file_op: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;

                // FOF_SILENT hides Windows' progress dialog (we render our own);
                // omitting FOF_NOCONFIRMATION keeps confirmation prompts.
                let allow_undo = match &kind {
                    FileOpKind::Delete { allow_undo } => *allow_undo,
                    FileOpKind::Move { .. } | FileOpKind::Copy { .. } => true,
                };
                let undo = if allow_undo { FOF_ALLOWUNDO.0 } else { 0 };
                // Keep collision confirmation enabled. In particular, do not
                // set FOF_RENAMEONCOLLISION: that flag silently chooses a new
                // name, while omitting it lets IFileOperation show the Shell's
                // conflict dialog with the user's available choices.
                file_op.SetOperationFlags(FILEOPERATION_FLAGS(FOF_SILENT.0 | undo))?;

                if owner_hwnd != 0 {
                    file_op.SetOwnerWindow(HWND(owner_hwnd as *mut core::ffi::c_void))?;
                }

                let base_total = sources.len() as u32;
                let sink: IFileOperationProgressSink = FileOpProgressSink {
                    tx: tx.clone(),
                    cancel: cancel.clone(),
                    notify: notify.clone(),
                    recorder: worker_recorder.clone(),
                    base_total,
                    pre_count: AtomicU32::new(0),
                    post_count: AtomicU32::new(0),
                }
                .into();
                let cookie = file_op.Advise(&sink)?;

                // Build the destination item once for move/copy operations.
                let dest_item: Option<IShellItem> = match &kind {
                    FileOpKind::Delete { .. } => None,
                    FileOpKind::Move { dest } | FileOpKind::Copy { dest } => {
                        let hdest = HSTRING::from(dest.to_string_lossy().as_ref());
                        Some(SHCreateItemFromParsingName(&hdest, None)?)
                    }
                };

                for path in &sources {
                    let hpath = HSTRING::from(path.to_string_lossy().as_ref());
                    let item: IShellItem = SHCreateItemFromParsingName(&hpath, None)?;
                    match &kind {
                        FileOpKind::Delete { .. } => {
                            file_op.DeleteItem(&item, None)?;
                        }
                        FileOpKind::Move { .. } => {
                            file_op.MoveItem(&item, dest_item.as_ref().unwrap(), None, None)?;
                        }
                        FileOpKind::Copy { .. } => {
                            file_op.CopyItem(&item, dest_item.as_ref().unwrap(), None, None)?;
                        }
                    }
                }

                file_op.PerformOperations()?;
                let _ = file_op.Unadvise(cookie);
                Ok(())
            })();

            let _ = tx.send(FileOpProgress::Finished {
                cancelled: cancel.load(Ordering::Relaxed),
                error: result.err().map(|e| e.to_string()),
            });
            notify();

            CoUninitialize();
        }
    });
    recorder
}

pub fn restore_paths_native(pidls: Vec<Vec<u8>>) -> windows::core::Result<()> {
    use windows::Win32::System::Com::{CoTaskMemAlloc, CoTaskMemFree};

    unsafe {
        for pidl_bytes in pidls {
            let pidl = CoTaskMemAlloc(pidl_bytes.len()) as *mut u8;
            if pidl.is_null() {
                return Err(Error::from(HRESULT(0x80004005u32 as i32)));
            }

            let result = (|| -> windows::core::Result<()> {
                std::ptr::copy_nonoverlapping(pidl_bytes.as_ptr(), pidl, pidl_bytes.len());

                let shell_item: IShellItem = SHCreateItemFromIDList(pidl as *const ITEMIDLIST)?;
                let context_menu: IContextMenu =
                    shell_item.BindToHandler(None, &BHID_SFUIObject)?;

                let restore_verb = PCSTR(b"undelete\0".as_ptr());
                let info = CMINVOKECOMMANDINFOEX {
                    cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32,
                    fMask: 0,
                    hwnd: HWND(std::ptr::null_mut()),
                    lpVerb: restore_verb,
                    lpVerbW: PCWSTR::null(),
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };

                context_menu.InvokeCommand(&info as *const _ as *const CMINVOKECOMMANDINFO)
            })();

            CoTaskMemFree(Some(pidl as _));
            result?;
        }
    }

    Ok(())
}

pub fn directory_child_paths(dir: &Path) -> HashSet<PathBuf> {
    read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn selection_paths_after_paste(
    target_dir: &Path,
    before_entries: &HashSet<PathBuf>,
    sources: &[PathBuf],
) -> Vec<PathBuf> {
    let mut pasted_paths: Vec<PathBuf> = directory_child_paths(target_dir)
        .difference(before_entries)
        .cloned()
        .collect();

    if pasted_paths.is_empty() {
        for source in sources {
            if let Some(name) = source.file_name() {
                let candidate = target_dir.join(name);
                if candidate.exists() {
                    pasted_paths.push(candidate);
                }
            }
        }
    }

    pasted_paths
}
