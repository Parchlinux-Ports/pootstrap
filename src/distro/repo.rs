use crate::error::{PootstrapError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LocalRepoManager;

impl LocalRepoManager {
    pub const DEFAULT_REPO_NAME: &'static str = "pootstrap-local";

    pub fn list_packages(repo_dir: &Path) -> Result<Vec<PathBuf>> {
        if !repo_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        let entries = fs::read_dir(repo_dir).map_err(PootstrapError::Io)?;

        for entry in entries.flatten() {
            let path = entry.path();
            let is_pkg = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.ends_with(".pkg.tar.zst") || name.ends_with(".pkg.tar.xz"));
            if is_pkg {
                packages.push(path);
            }
        }

        packages.sort();
        Ok(packages)
    }

    pub fn update_repo(repo_dir: &Path, repo_name: Option<&str>) -> Result<PathBuf> {
        which::which("repo-add").map_err(|_| PootstrapError::MissingTool {
            tool: "repo-add (from pacman)".into(),
        })?;

        if !repo_dir.exists() {
            fs::create_dir_all(repo_dir).map_err(PootstrapError::Io)?;
        }

        let name = repo_name.unwrap_or(Self::DEFAULT_REPO_NAME);
        let db_path = repo_dir.join(format!("{}.db.tar.gz", name));

        let packages = Self::list_packages(repo_dir)?;
        if packages.is_empty() {
            tracing::warn!("No package files found in {:?} to index", repo_dir);
            return Ok(db_path);
        }

        tracing::info!(
            "Indexing {} packages into local repository database {:?}",
            packages.len(),
            db_path
        );

        let mut cmd = Command::new("repo-add");
        cmd.arg("-n").arg("-R").arg(&db_path);

        for pkg in &packages {
            cmd.arg(pkg);
        }

        let status = cmd.status().map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::CommandFailed {
                command: format!("repo-add {:?}", db_path),
                exit_code: status.code(),
                stderr: "repo-add failed to index packages".into(),
            });
        }

        tracing::info!("Local repository index updated successfully at {:?}", db_path);
        Ok(db_path)
    }
}
