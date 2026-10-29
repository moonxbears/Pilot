//! Milestone 27 & Milestone 43.3: Native Package Archive Packer and Unpacker (`.pilpkg`).

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

const PILPKG_MAGIC: &[u8; 6] = b"PILPKG";
const PILPKG_VERSION: u8 = 1;

/// Packs a project directory into a `.pilpkg` archive.
pub fn pack_directory(project_dir: &str, output_pkg_path: &str) -> io::Result<usize> {
    let mut files_to_pack = Vec::new();
    collect_files(Path::new(project_dir), Path::new(project_dir), &mut files_to_pack)?;

    let mut out_file = File::create(output_pkg_path)?;
    out_file.write_all(PILPKG_MAGIC)?;
    out_file.write_all(&[PILPKG_VERSION])?;

    let file_count = files_to_pack.len() as u32;
    out_file.write_all(&file_count.to_le_bytes())?;

    for (rel_path, abs_path) in &files_to_pack {
        let content = fs::read(abs_path)?;
        let path_bytes = rel_path.as_bytes();
        let path_len = path_bytes.len() as u32;
        let content_len = content.len() as u64;

        out_file.write_all(&path_len.to_le_bytes())?;
        out_file.write_all(path_bytes)?;
        out_file.write_all(&content_len.to_le_bytes())?;
        out_file.write_all(&content)?;
    }

    out_file.flush()?;
    Ok(files_to_pack.len())
}

/// Unpacks a `.pilpkg` archive into a target destination directory.
pub fn unpack_archive(pkg_path: &str, target_dir: &str) -> io::Result<usize> {
    let mut file = File::open(pkg_path)?;
    let mut magic = [0u8; 6];
    file.read_exact(&mut magic)?;
    if &magic != PILPKG_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid .pilpkg magic header"));
    }

    let mut ver = [0u8; 1];
    file.read_exact(&mut ver)?;
    if ver[0] != PILPKG_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unsupported .pilpkg version"));
    }

    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf)?;
    let file_count = u32::from_le_bytes(count_buf) as usize;

    let target_root = Path::new(target_dir);
    fs::create_dir_all(target_root)?;

    for _ in 0..file_count {
        let mut path_len_buf = [0u8; 4];
        file.read_exact(&mut path_len_buf)?;
        let path_len = u32::from_le_bytes(path_len_buf) as usize;

        let mut path_bytes = vec![0u8; path_len];
        file.read_exact(&mut path_bytes)?;
        let rel_path = String::from_utf8_lossy(&path_bytes);

        let mut content_len_buf = [0u8; 8];
        file.read_exact(&mut content_len_buf)?;
        let content_len = u64::from_le_bytes(content_len_buf) as usize;

        let mut content = vec![0u8; content_len];
        file.read_exact(&mut content)?;

        let dest_path = target_root.join(rel_path.as_ref());
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest_path, content)?;
    }

    Ok(file_count)
}

fn collect_files(root: &Path, current: &Path, list: &mut Vec<(String, std::path::PathBuf)>) -> io::Result<()> {
    if current.is_dir() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip .git, target, node_modules, .venv
            if name == ".git" || name == "target" || name == ".venv" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                collect_files(root, &path, list)?;
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    list.push((rel.to_string_lossy().to_string(), path.clone()));
                }
            }
        }
    }
    Ok(())
}
