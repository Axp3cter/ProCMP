//! What counts as a build input, derived from the plan.

use std::collections::{BTreeMap, BTreeSet};

use wax::{Glob, Program};

use crate::digest::{self, Digest, Hasher};
use crate::error::{Error, Result};
use crate::path::AbsPath;
use crate::plan::Graph;

/// Version-control metadata, never an input.
const VCS: &str = ".git";

/// Every file under every root counts, whatever its extension. A content loader can
/// make a `.json` or a `.png` a real input.
#[derive(Debug)]
pub struct Scope {
    roots: Vec<AbsPath>,
    excluded: BTreeSet<AbsPath>,
    ignore: Vec<Glob<'static>>,
}

impl Scope {
    pub fn of(graph: &Graph, cache: &AbsPath) -> Result<Self> {
        let mut roots = Vec::new();
        let mut excluded = BTreeSet::from([cache.clone()]);
        let mut patterns: BTreeSet<&str> = BTreeSet::new();

        for task in &graph.tasks {
            roots.extend(task.sources.iter().cloned());
            excluded.insert(task.output.clone());
            patterns.extend(task.ignore.iter().map(String::as_str));
        }

        roots.sort();
        roots.dedup();

        // A nested root would hash its files twice under two relative names.
        let outer = roots.clone();
        roots.retain(|root| !outer.iter().any(|other| root.is_within(other)));

        let ignore = patterns
            .into_iter()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(Glob::into_owned)
                    .map_err(|e| Error::BadGlob(pattern.to_owned(), e.to_string()))
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            roots,
            excluded,
            ignore,
        })
    }

    pub fn roots(&self) -> &[AbsPath] {
        &self.roots
    }

    /// A path that is not UTF-8 is outside the project by construction.
    pub fn contains(&self, path: &std::path::Path) -> bool {
        path.to_str()
            .and_then(|text| AbsPath::new(text).ok())
            .is_some_and(|path| self.holds(&path))
    }

    /// Sorted, so filesystems that enumerate differently agree. A root that does not
    /// exist contributes nothing.
    pub fn fingerprint(&self) -> Result<Digest> {
        let mut entries: BTreeMap<String, Digest> = BTreeMap::new();

        for root in &self.roots {
            self.walk(root, root, &mut entries)?;
        }

        let mut h = Hasher::new();
        h.seq(
            "sources",
            entries
                .iter()
                .map(|(path, d)| format!("{path}:{}", d.hex())),
        );
        Ok(h.finish())
    }

    fn holds(&self, path: &AbsPath) -> bool {
        let Some(root) = self.roots.iter().find(|root| path.is_under(root)) else {
            return false;
        };

        if self.excluded.iter().any(|excluded| path.is_under(excluded)) {
            return false;
        }

        let relative = path.relative_to(root);
        if relative.split('/').any(|segment| segment == VCS) {
            return false;
        }

        !self
            .ignore
            .iter()
            .any(|glob| glob.is_match(relative.as_str()))
    }

    fn walk(
        &self,
        root: &AbsPath,
        dir: &AbsPath,
        entries: &mut BTreeMap<String, Digest>,
    ) -> Result<()> {
        let read = match std::fs::read_dir(dir.as_std()) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(Error::Read(dir.to_string(), e.to_string())),
        };

        for entry in read {
            let entry = entry.map_err(|e| Error::Read(dir.to_string(), e.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();

            let Ok(path) = dir.join(&name) else { continue };
            if !self.holds(&path) {
                continue;
            }

            let kind = entry
                .file_type()
                .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;

            if kind.is_dir() {
                self.walk(root, &path, entries)?;
                continue;
            }

            // Symlinks are skipped: following them could leave the project.
            if !kind.is_file() {
                continue;
            }

            let bytes = std::fs::read(path.as_std())
                .map_err(|e| Error::Read(path.to_string(), e.to_string()))?;
            entries.insert(path.relative_to(root), digest::of(&bytes));
        }

        Ok(())
    }
}
