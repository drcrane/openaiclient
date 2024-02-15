use std::fs;
use std::path::{Path, PathBuf};
use serde::de::DeserializeOwned;
use serde::{Serialize};
use std::string;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Error, ErrorKind};
use std::process;
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HelperError {
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
	#[error("Serde error: {0}")]
	Serde(#[from] serde_json::Error),
	#[error("FromUtf8 error: {0}")]
	FromUtf8Error(#[from] string::FromUtf8Error),
}

pub fn has_specific_extension<P: AsRef<Path>>(path: P, ext: &str) -> bool {
	match path.as_ref().extension() {
		Some(os_str) => os_str == ext,
		None => false,
	}
}

pub fn config_get_dir(name: Option<&str>) -> Result<PathBuf, std::io::Error> {
	let xdg_config_home_env = env::var("XDG_CONFIG_HOME");
	let mut pb = if let Ok(config_home) = xdg_config_home_env {
		PathBuf::from(config_home)
	} else {
		let home_env = env::var("HOME");
		if let Ok(home) = home_env {
			let mut config_home = PathBuf::from(home);
			config_home.push(".config");
			config_home
		} else {
			return Err(std::io::Error::new(std::io::ErrorKind::Other, "XDG_CONFIG_HOME or HOME not found"))
		}
	};
	if let Some(app_name) = name {
		pb.push(app_name);
	}
	println!("{}", pb.display());
	Ok(pb)
}

pub fn config_load<T: DeserializeOwned>(app_name: &str, config_name: &str) -> Result<T, HelperError> {
	let mut config_file = config_get_dir(Some(app_name))?;
	config_file.push(config_name.to_string() + ".json");
	read_from_json(&config_file)
}

pub fn config_save<T: Serialize>(app_name: &str, config_name: &str, object: &T) -> Result<(), HelperError> {
	let mut config_file = config_get_dir(Some(app_name))?;
	fs::create_dir_all(&config_file)?;
	config_file.push(config_name.to_string() + ".json");
	save_to_json(&config_file, object)
}

pub fn read_from_json<T: DeserializeOwned>(file_path: impl AsRef<Path>) -> Result<T, HelperError> {
	let mut file = File::open(file_path.as_ref())?;
	let mut content = String::new();
	file.read_to_string(&mut content)?;
	let parsed_json: T = serde_json::from_str(&content)?;
	Ok(parsed_json)
}

pub fn save_to_json<T: Serialize>(file_path: impl AsRef<Path>, object: &T) -> Result<(), HelperError> {
	let serialised = serde_json::to_string_pretty(&object)?;
	let mut file = OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path.as_ref())?;
	writeln!(file, "{}", &serialised)?;
	Ok(())
}

pub fn list_files<F>(dir: &Path, accept: F, depth: usize) -> Result<Vec<PathBuf>, io::Error> where F: Fn(PathBuf) -> Option<PathBuf> {
	let mut files_list = Vec::new();
	let mut stack = Vec::new();
	stack.push(dir.to_path_buf());

	while let Some(current_dir) = stack.pop() {
		for entry in fs::read_dir(current_dir)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				if (depth > stack.len()) {
					stack.push(path.clone());
				}
				match accept(path) {
					Some(result_path) => files_list.push(result_path),
					None => {}
				}
			} else if path.is_file() {
				match accept(path) {
					Some(result_path) => files_list.push(result_path),
					None => {}
				}
			}
		}
	}
	Ok(files_list)
}

pub fn extract_zip_file_with_password(dest_path: &Path, file_path: &Path, password: &str) -> Result<(), HelperError> {
	let password_arg = "-p".to_owned() + &password;
	let file_arg = file_path.to_str().unwrap();
	let dest_arg = "-o".to_owned() + dest_path.to_str().unwrap();
	println!("Extracting {} with password: {}", &file_arg, &password_arg);
	let process = match process::Command::new("./7z.sh")
			.args(&["x", "-y", &dest_arg, &password_arg, &file_arg])
			.spawn() {
		Ok(process) => process,
		Err(err) => return Err(HelperError::Io(err)),
	};
	let output = match process.wait_with_output() {
		Ok(output) => output,
		Err(err) => return Err(HelperError::Io(err)),
	};
	let stdout = match std::string::String::from_utf8(output.stdout) {
		Ok(stdout) => stdout,
		Err(err) => return Err(HelperError::FromUtf8Error(err)),
	};
	let stderr = match std::string::String::from_utf8(output.stderr) {
		Ok(stderr) => stderr,
		Err(err) => return Err(HelperError::FromUtf8Error(err)),
	};
	println!("{}", stdout);
	eprintln!("{}", stderr);
	Ok(())
}

