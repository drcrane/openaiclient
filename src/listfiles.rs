#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
mod tools;
mod helpers;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use regex::Regex;
use std::ffi::OsString;
use tools::files::make_gitaccept_matcher;

pub fn list_files<F>(dir: &Path, accept: &F, depth: usize) -> Result<Vec<PathBuf>, io::Error>
where
	F: Fn(PathBuf) -> Option<PathBuf>,
{
	let mut result = Vec::new();
	list_files_recursive(dir, &accept, depth, &mut result)?;
	Ok(result)
}

fn list_files_recursive<F>(dir: &Path, accept: &F, depth: usize, result: &mut Vec<PathBuf>) -> Result<(), io::Error>
where
	F: Fn(PathBuf) -> Option<PathBuf>,
{
	if depth == 0 {
		return Ok(());
	}

	for entry in fs::read_dir(dir)? {
		let entry = entry?;
		let path = entry.path();
		if path.is_dir() {
			list_files_recursive(&path, accept, depth - 1, result)?;
		} else if let Some(accepted_path) = accept(path.clone()) {
			result.push(accepted_path);
		}
	}
	Ok(())
}

fn read_gitignore(dir: &Path) -> Result<Vec<Regex>, io::Error> {
	let gitignore_path = dir.join(".gitignore");
	if !gitignore_path.exists() {
		return Ok(Vec::new());
	}

	let contents = fs::read_to_string(gitignore_path)?;
	let mut patterns = Vec::new();
	for line in contents.lines() {
		if !line.starts_with('#') && !line.trim().is_empty() {
			let pattern = Regex::new(&format!("^{}$", line.trim())).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid regex pattern"))?;
			patterns.push(pattern);
		}
	}
	Ok(patterns)
}

fn create_accept_function(dir: &Path) -> Result<impl Fn(&Path) -> Option<PathBuf>, io::Error> {
	Ok(|path: &Path| {
		match fs::metadata(&path) {
			Ok(metadata) => {
				if metadata.is_dir() {
					None
				} else {
					Some(PathBuf::from(path))
				}
			},
			Err(_) => {
				None
			},
		}
	})
}

fn main() -> Result<(), io::Error> {
	let dir = Path::new(".");
	//let accept = if let Ok(gitignore) = fs::read_to_string(".gitignore") {
	//	make_gitaccept_matcher(&gitignore)
	//} else {
	//	create_accept_function(dir)?
	//};
    let accept = make_gitaccept_matcher(&fs::read_to_string(".gitignore")?);
	let files = helpers::list_files(dir, &accept, 2)?;
	for file in files {
		match file.strip_prefix(dir) {
			Ok(remaining_path) => {
				println!("{}", remaining_path.display());
			},
			Err(err) => {
				//println!("Error: {}", err);
			}
		}
	}
	Ok(())
}

