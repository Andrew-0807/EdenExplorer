//! Undo history for file operations: a capped stack of steps and the pure
//! decisions around them (inverse construction, preflight checks). Running
//! the inverse against the shell lives in the window code.
//!
//! One step is one user action, so a multi-item move or delete is undone by a
//! single Ctrl+Z. Undo only: there is no redo.

use std::path::{Path, PathBuf};

/// Steps kept per window. The oldest is dropped when a new one would exceed it.
pub const UNDO_LIMIT: usize = 50;

/// What the user did. Selects the toast text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoKind {
    Rename,
    Move,
    NewFolder,
    NewFile,
    Copy,
    Delete,
}

/// An item sent to the Recycle Bin and the Bin entry that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashedItem {
    pub original: PathBuf,
    /// Absolute PIDL of the Bin entry, as `restore_paths_native` expects.
    pub bin_pidl: Vec<u8>,
}

/// One thing that was done. Paths are the final names reported by the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoOp {
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    /// `(old, new)` per item.
    Move {
        pairs: Vec<(PathBuf, PathBuf)>,
    },
    /// New folder, new file or paste-copy.
    Create {
        paths: Vec<PathBuf>,
    },
    Trash {
        items: Vec<TrashedItem>,
    },
}

impl UndoOp {
    fn is_empty(&self) -> bool {
        match self {
            UndoOp::Rename { .. } => false,
            UndoOp::Move { pairs } => pairs.is_empty(),
            UndoOp::Create { paths } => paths.is_empty(),
            UndoOp::Trash { items } => items.is_empty(),
        }
    }

    fn item_count(&self) -> usize {
        match self {
            UndoOp::Rename { .. } => 1,
            UndoOp::Move { pairs } => pairs.len(),
            UndoOp::Create { paths } => paths.len(),
            UndoOp::Trash { items } => items.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoStep {
    pub kind: UndoKind,
    pub ops: Vec<UndoOp>,
}

impl UndoStep {
    /// `None` when there is nothing to undo, so empty steps never reach the stack.
    pub fn new(kind: UndoKind, ops: Vec<UndoOp>) -> Option<Self> {
        let ops: Vec<UndoOp> = ops.into_iter().filter(|op| !op.is_empty()).collect();
        if ops.is_empty() {
            None
        } else {
            Some(Self { kind, ops })
        }
    }

    pub fn item_count(&self) -> usize {
        self.ops.iter().map(UndoOp::item_count).sum()
    }

    /// File name of the first item as it is named right now, for the toast.
    pub fn subject(&self) -> String {
        let path = match self.ops.first() {
            Some(UndoOp::Rename { to, .. }) => Some(to),
            Some(UndoOp::Move { pairs }) => pairs.first().map(|(_, new)| new),
            Some(UndoOp::Create { paths }) => paths.first(),
            Some(UndoOp::Trash { items }) => items.first().map(|item| &item.original),
            None => None,
        };
        path.and_then(|p| p.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

/// One shell call that reverses part of a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoAction {
    /// Give `from` the file name of `to`, keeping its folder.
    Rename { from: PathBuf, to: PathBuf },
    /// Move `from` back so it ends up at the full path `to`.
    MoveBack { from: PathBuf, to: PathBuf },
    /// Send to the Recycle Bin, never delete permanently.
    Trash { paths: Vec<PathBuf> },
    /// Restore from the Recycle Bin.
    Restore { items: Vec<TrashedItem> },
}

/// Why a step cannot be undone right now. Nothing is changed in that case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocked {
    /// A file the undo needs is gone.
    Missing(PathBuf),
    /// Undoing would land on a name that is in use.
    Taken(PathBuf),
}

fn same_name_ignoring_case(a: &Path, b: &Path) -> bool {
    a.parent() == b.parent()
        && a.file_name().map(|n| n.to_ascii_lowercase())
            == b.file_name().map(|n| n.to_ascii_lowercase())
}

/// Builds the inverse of `step`, newest op first. Every source must exist and
/// every target name must be free, otherwise the whole step is refused (no
/// partial undo). `exists` is injected so the decision can be tested.
pub fn plan_undo(
    step: &UndoStep,
    exists: impl Fn(&Path) -> bool,
) -> Result<Vec<UndoAction>, Blocked> {
    let mut actions = Vec::new();
    for op in step.ops.iter().rev() {
        match op {
            UndoOp::Rename { from, to } => {
                if !exists(to) {
                    return Err(Blocked::Missing(to.clone()));
                }
                // A case-only rename finds the old name "taken" by the file itself.
                if exists(from) && !same_name_ignoring_case(from, to) {
                    return Err(Blocked::Taken(from.clone()));
                }
                actions.push(UndoAction::Rename {
                    from: to.clone(),
                    to: from.clone(),
                });
            }
            UndoOp::Move { pairs } => {
                for (old, new) in pairs.iter().rev() {
                    if !exists(new) {
                        return Err(Blocked::Missing(new.clone()));
                    }
                    if exists(old) {
                        return Err(Blocked::Taken(old.clone()));
                    }
                    if let Some(parent) = old.parent()
                        && !exists(parent)
                    {
                        return Err(Blocked::Missing(parent.to_path_buf()));
                    }
                    actions.push(UndoAction::MoveBack {
                        from: new.clone(),
                        to: old.clone(),
                    });
                }
            }
            UndoOp::Create { paths } => {
                if let Some(missing) = paths.iter().find(|p| !exists(p)) {
                    return Err(Blocked::Missing(missing.clone()));
                }
                actions.push(UndoAction::Trash {
                    paths: paths.clone(),
                });
            }
            UndoOp::Trash { items } => {
                if let Some(item) = items.iter().find(|item| exists(&item.original)) {
                    return Err(Blocked::Taken(item.original.clone()));
                }
                actions.push(UndoAction::Restore {
                    items: items.clone(),
                });
            }
        }
    }
    Ok(actions)
}

/// Newest-last history of undoable steps.
#[derive(Default)]
pub struct UndoStack {
    steps: Vec<UndoStep>,
}

impl UndoStack {
    pub fn push(&mut self, step: UndoStep) {
        if self.steps.len() >= UNDO_LIMIT {
            self.steps.remove(0);
        }
        self.steps.push(step);
    }

    /// Records a rename. Naming a just-created folder or file folds into the
    /// creation, so one Ctrl+Z removes the item instead of reverting its name.
    pub fn push_rename(&mut self, from: PathBuf, to: PathBuf) {
        if let Some(UndoStep {
            kind: UndoKind::NewFolder | UndoKind::NewFile,
            ops,
        }) = self.steps.last_mut()
            && let [UndoOp::Create { paths }] = ops.as_mut_slice()
            && let [path] = paths.as_mut_slice()
            && *path == from
        {
            *path = to;
            return;
        }
        self.push(UndoStep {
            kind: UndoKind::Rename,
            ops: vec![UndoOp::Rename { from, to }],
        });
    }

    pub fn last(&self) -> Option<&UndoStep> {
        self.steps.last()
    }

    pub fn pop(&mut self) -> Option<UndoStep> {
        self.steps.pop()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn rename_step(from: &str, to: &str) -> UndoStep {
        UndoStep::new(
            UndoKind::Rename,
            vec![UndoOp::Rename {
                from: p(from),
                to: p(to),
            }],
        )
        .unwrap()
    }

    fn exists_in(paths: &[&str]) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = paths.iter().map(|s| p(s)).collect();
        move |path| set.contains(path)
    }

    #[test]
    fn stack_drops_oldest_beyond_limit() {
        let mut stack = UndoStack::default();
        for i in 0..UNDO_LIMIT + 5 {
            stack.push(rename_step(&format!("C:\\a{i}"), &format!("C:\\b{i}")));
        }
        assert_eq!(stack.len(), UNDO_LIMIT);
        assert_eq!(
            stack.last().unwrap().subject(),
            format!("b{}", UNDO_LIMIT + 4)
        );
        let mut oldest = None;
        while let Some(step) = stack.pop() {
            oldest = Some(step);
        }
        assert_eq!(oldest.unwrap().subject(), "b5");
    }

    #[test]
    fn empty_steps_are_not_created() {
        assert!(UndoStep::new(UndoKind::Move, vec![]).is_none());
        assert!(UndoStep::new(UndoKind::Move, vec![UndoOp::Move { pairs: vec![] }]).is_none());
        assert!(UndoStep::new(UndoKind::Copy, vec![UndoOp::Create { paths: vec![] }]).is_none());
    }

    #[test]
    fn multi_item_move_is_one_step_with_all_items() {
        let step = UndoStep::new(
            UndoKind::Move,
            vec![UndoOp::Move {
                pairs: vec![
                    (p("C:\\a\\x"), p("C:\\b\\x")),
                    (p("C:\\a\\y"), p("C:\\b\\y")),
                    (p("C:\\a\\z"), p("C:\\b\\z")),
                ],
            }],
        )
        .unwrap();
        let mut stack = UndoStack::default();
        stack.push(step);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.last().unwrap().item_count(), 3);
        assert_eq!(stack.last().unwrap().subject(), "x");
    }

    #[test]
    fn rename_inverse_renames_back() {
        let step = rename_step("C:\\d\\old.txt", "C:\\d\\new.txt");
        let actions = plan_undo(&step, exists_in(&["C:\\d\\new.txt"])).unwrap();
        assert_eq!(
            actions,
            vec![UndoAction::Rename {
                from: p("C:\\d\\new.txt"),
                to: p("C:\\d\\old.txt"),
            }]
        );
    }

    #[test]
    fn case_only_rename_can_be_undone() {
        let step = rename_step("C:\\d\\a.txt", "C:\\d\\A.txt");
        // The file system reports both spellings as existing.
        assert!(plan_undo(&step, exists_in(&["C:\\d\\A.txt", "C:\\d\\a.txt"])).is_ok());
    }

    #[test]
    fn rename_blocked_when_target_missing_or_old_name_taken() {
        let step = rename_step("C:\\d\\old.txt", "C:\\d\\new.txt");
        assert_eq!(
            plan_undo(&step, exists_in(&[])),
            Err(Blocked::Missing(p("C:\\d\\new.txt")))
        );
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\d\\new.txt", "C:\\d\\old.txt"])),
            Err(Blocked::Taken(p("C:\\d\\old.txt")))
        );
    }

    #[test]
    fn move_inverse_goes_back_newest_first_and_keeps_old_names() {
        let step = UndoStep::new(
            UndoKind::Move,
            vec![UndoOp::Move {
                pairs: vec![
                    (p("C:\\a\\x"), p("C:\\b\\x")),
                    (p("C:\\a\\y"), p("C:\\b\\y (2)")),
                ],
            }],
        )
        .unwrap();
        let actions = plan_undo(&step, exists_in(&["C:\\b\\x", "C:\\b\\y (2)", "C:\\a"])).unwrap();
        assert_eq!(
            actions,
            vec![
                UndoAction::MoveBack {
                    from: p("C:\\b\\y (2)"),
                    to: p("C:\\a\\y"),
                },
                UndoAction::MoveBack {
                    from: p("C:\\b\\x"),
                    to: p("C:\\a\\x"),
                },
            ]
        );
    }

    #[test]
    fn one_bad_item_aborts_the_whole_step() {
        let step = UndoStep::new(
            UndoKind::Move,
            vec![UndoOp::Move {
                pairs: vec![
                    (p("C:\\a\\x"), p("C:\\b\\x")),
                    (p("C:\\a\\y"), p("C:\\b\\y")),
                ],
            }],
        )
        .unwrap();
        // y was already moved away by something else.
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\b\\x", "C:\\a"])),
            Err(Blocked::Missing(p("C:\\b\\y")))
        );
        // x's old spot is occupied.
        assert_eq!(
            plan_undo(
                &step,
                exists_in(&["C:\\b\\x", "C:\\b\\y", "C:\\a", "C:\\a\\x"])
            ),
            Err(Blocked::Taken(p("C:\\a\\x")))
        );
    }

    #[test]
    fn move_blocked_when_old_folder_is_gone() {
        let step = UndoStep::new(
            UndoKind::Move,
            vec![UndoOp::Move {
                pairs: vec![(p("C:\\a\\x"), p("C:\\b\\x"))],
            }],
        )
        .unwrap();
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\b\\x"])),
            Err(Blocked::Missing(p("C:\\a")))
        );
    }

    #[test]
    fn create_and_copy_inverse_trash_the_new_items() {
        let step = UndoStep::new(
            UndoKind::Copy,
            vec![UndoOp::Create {
                paths: vec![p("C:\\d\\a (2)"), p("C:\\d\\b")],
            }],
        )
        .unwrap();
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\d\\a (2)", "C:\\d\\b"])).unwrap(),
            vec![UndoAction::Trash {
                paths: vec![p("C:\\d\\a (2)"), p("C:\\d\\b")],
            }]
        );
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\d\\b"])),
            Err(Blocked::Missing(p("C:\\d\\a (2)")))
        );
    }

    #[test]
    fn trash_inverse_restores_unless_original_name_is_taken() {
        let item = TrashedItem {
            original: p("C:\\d\\gone.txt"),
            bin_pidl: vec![1, 2, 3],
        };
        let step = UndoStep::new(
            UndoKind::Delete,
            vec![UndoOp::Trash {
                items: vec![item.clone()],
            }],
        )
        .unwrap();
        assert_eq!(
            plan_undo(&step, exists_in(&[])).unwrap(),
            vec![UndoAction::Restore { items: vec![item] }]
        );
        assert_eq!(
            plan_undo(&step, exists_in(&["C:\\d\\gone.txt"])),
            Err(Blocked::Taken(p("C:\\d\\gone.txt")))
        );
    }

    #[test]
    fn mixed_step_undoes_ops_in_reverse_order() {
        let step = UndoStep::new(
            UndoKind::Move,
            vec![
                UndoOp::Move {
                    pairs: vec![(p("C:\\a\\x"), p("C:\\b\\x"))],
                },
                UndoOp::Create {
                    paths: vec![p("C:\\b\\c")],
                },
            ],
        )
        .unwrap();
        let actions = plan_undo(&step, exists_in(&["C:\\b\\x", "C:\\b\\c", "C:\\a"])).unwrap();
        assert!(matches!(actions[0], UndoAction::Trash { .. }));
        assert!(matches!(actions[1], UndoAction::MoveBack { .. }));
    }

    #[test]
    fn naming_a_new_folder_folds_into_its_creation() {
        let mut stack = UndoStack::default();
        stack.push(
            UndoStep::new(
                UndoKind::NewFolder,
                vec![UndoOp::Create {
                    paths: vec![p("C:\\d\\New Folder")],
                }],
            )
            .unwrap(),
        );
        stack.push_rename(p("C:\\d\\New Folder"), p("C:\\d\\Photos"));
        assert_eq!(stack.len(), 1);
        let step = stack.last().unwrap();
        assert_eq!(step.kind, UndoKind::NewFolder);
        assert_eq!(step.subject(), "Photos");
    }

    #[test]
    fn rename_of_something_else_is_its_own_step() {
        let mut stack = UndoStack::default();
        stack.push(
            UndoStep::new(
                UndoKind::NewFolder,
                vec![UndoOp::Create {
                    paths: vec![p("C:\\d\\New Folder")],
                }],
            )
            .unwrap(),
        );
        stack.push_rename(p("C:\\d\\other"), p("C:\\d\\renamed"));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.last().unwrap().kind, UndoKind::Rename);
    }
}
