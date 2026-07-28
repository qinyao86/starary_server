use anyhow::{bail, Context};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug)]
pub struct FileMigrationPlan {
    pub source: PathBuf,
    pub destination: PathBuf,
}

pub type MigrationManifest = HashMap<PathBuf, FileStamp>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileStamp {
    length: u64,
    modified: Option<SystemTime>,
}

pub fn prepare_and_copy(plans: &[FileMigrationPlan]) -> anyhow::Result<MigrationManifest> {
    for plan in plans {
        validate_source(&plan.source)?;
        validate_empty_destination(&plan.destination)?;
    }
    synchronize(plans, false, None)
}

pub fn synchronize(
    plans: &[FileMigrationPlan],
    remove_extraneous: bool,
    previous_manifest: Option<&MigrationManifest>,
) -> anyhow::Result<MigrationManifest> {
    let mut manifest = MigrationManifest::new();
    for plan in plans {
        sync_directory(
            &plan.source,
            &plan.destination,
            remove_extraneous,
            previous_manifest,
            &mut manifest,
        )?;
    }
    Ok(manifest)
}

fn validate_source(path: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("source library folder is unavailable: {}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "source library folder must be a real directory: {}",
            path.display()
        );
    }
    Ok(())
}

fn validate_empty_destination(path: &Path) -> anyhow::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "destination library folder cannot be inspected: {}",
                    path.display()
                )
            });
        }
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "destination library folder must be a real directory: {}",
            path.display()
        );
    }
    if fs::read_dir(path)
        .with_context(|| {
            format!(
                "destination library folder cannot be read: {}",
                path.display()
            )
        })?
        .next()
        .is_some()
    {
        bail!(
            "destination library folder must be empty: {}",
            path.display()
        );
    }
    Ok(())
}

fn sync_directory(
    source: &Path,
    destination: &Path,
    remove_extraneous: bool,
    previous_manifest: Option<&MigrationManifest>,
    manifest: &mut MigrationManifest,
) -> anyhow::Result<()> {
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "could not create destination folder: {}",
            destination.display()
        )
    })?;
    let mut source_names = HashSet::<OsString>::new();

    for entry in fs::read_dir(source)
        .with_context(|| format!("could not read source folder: {}", source.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        source_names.insert(name.clone());
        let source_path = entry.path();
        let destination_path = destination.join(&name);
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            bail!(
                "symbolic links are not supported during migration: {}",
                source_path.display()
            );
        }

        if metadata.is_dir() {
            remove_type_mismatch(&destination_path, true)?;
            sync_directory(
                &source_path,
                &destination_path,
                remove_extraneous,
                previous_manifest,
                manifest,
            )?;
        } else if metadata.is_file() {
            remove_type_mismatch(&destination_path, false)?;
            let current_stamp = file_stamp(&metadata);
            let unchanged = previous_manifest
                .and_then(|previous| previous.get(&source_path))
                .is_some_and(|previous| *previous == current_stamp)
                && destination_path.is_file();
            let final_stamp = if unchanged {
                current_stamp
            } else {
                copy_file_stable(&source_path, &destination_path)?
            };
            manifest.insert(source_path, final_stamp);
        } else {
            bail!(
                "unsupported file type during migration: {}",
                source_path.display()
            );
        }
    }

    if remove_extraneous {
        for entry in fs::read_dir(destination)? {
            let entry = entry?;
            if !source_names.contains(&entry.file_name()) {
                remove_destination_entry(&entry.path())?;
            }
        }
    }
    Ok(())
}

fn copy_file_stable(source: &Path, destination: &Path) -> anyhow::Result<FileStamp> {
    for _ in 0..3 {
        let before = file_stamp(&fs::metadata(source)?);
        fs::copy(source, destination).with_context(|| {
            format!(
                "could not copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        let after = file_stamp(&fs::metadata(source)?);
        if before == after {
            return Ok(after);
        }
    }
    bail!(
        "source file kept changing during migration: {}",
        source.display()
    )
}

fn file_stamp(metadata: &fs::Metadata) -> FileStamp {
    FileStamp {
        length: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

fn remove_type_mismatch(path: &Path, expect_directory: bool) -> anyhow::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || (expect_directory && !metadata.is_dir()) {
        fs::remove_file(path)?;
    } else if !expect_directory && metadata.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn remove_destination_entry(path: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn copies_and_mirrors_a_library_directory() {
        let base = std::env::temp_dir().join(format!("starary-migration-{}", Uuid::new_v4()));
        let source = base.join("source");
        let destination = base.join("destination");
        fs::create_dir_all(source.join(".starary/thumbs")).unwrap();
        fs::create_dir_all(source.join("assets")).unwrap();
        fs::write(source.join("assets/item.bin"), b"source").unwrap();
        fs::write(source.join(".starary/thumbs/item.webp"), b"thumb").unwrap();

        let plan = FileMigrationPlan {
            source: source.clone(),
            destination: destination.clone(),
        };
        let manifest = prepare_and_copy(std::slice::from_ref(&plan)).unwrap();
        assert_eq!(
            fs::read(destination.join("assets/item.bin")).unwrap(),
            b"source"
        );

        fs::write(source.join("assets/item.bin"), b"updated").unwrap();
        fs::write(destination.join("stale.bin"), b"stale").unwrap();
        synchronize(&[plan], true, Some(&manifest)).unwrap();
        assert_eq!(
            fs::read(destination.join("assets/item.bin")).unwrap(),
            b"updated"
        );
        assert!(!destination.join("stale.bin").exists());

        fs::remove_dir_all(base).unwrap();
    }
}
