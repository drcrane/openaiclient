#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use serde::de::DeserializeOwned;
use serde::Serialize;
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
	#[error("Other Error {0}")]
	FromString(String),
}

impl HelperError {
	pub fn msg<M: Into<String>>(msg: M) -> Self {
		HelperError::FromString(msg.into())
	}
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

pub fn list_files<F>(dir: &Path, mut accept: F, depth: usize) -> Result<Vec<PathBuf>, io::Error> where F: FnMut(&Path) -> Option<PathBuf> {
	let mut files_list = Vec::new();
	let mut stack = Vec::new();
	stack.push((dir.to_path_buf(), 0));

	while let Some((current_dir, current_depth)) = stack.pop() {
		for entry in fs::read_dir(current_dir)? {
			let curr_entry = entry?;
			let path = curr_entry.path();
			let file_type = curr_entry.file_type()?;
			if current_depth < depth {
				if file_type.is_dir() {
					stack.push((path.clone(), current_depth + 1));
				}
				if let Some(result_path) = accept(&path) {
					files_list.push(result_path)
				}
			}
		}
	}
	Ok(files_list)
}

pub fn extract_zip_file_with_password(extractor: &str, dest_path: &Path, file_path: &Path, password: &str) -> Result<(), HelperError> {
	let password_arg = "-p".to_owned() + &password;
	let file_arg = file_path.to_str().unwrap();
	let dest_arg = "-o".to_owned() + dest_path.to_str().unwrap();
	println!("Extracting {} with password: {}", &file_arg, &password_arg);
	let process = match process::Command::new(extractor)
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

const MAX_READ_BYTES: usize = 32_768;

pub trait FromInputBytes: Sized {
	fn from_bytes(bytes: Vec<u8>) -> Result<Self, HelperError>;
}

impl FromInputBytes for Vec<u8> {
	fn from_bytes(bytes: Vec<u8>) -> Result<Self, HelperError> {
		Ok(bytes)
	}
}

impl FromInputBytes for String {
	fn from_bytes(bytes: Vec<u8>) -> Result<Self, HelperError> {
		Ok(String::from_utf8(bytes)?)
	}
}

pub fn read_stdin<T>() -> Result<T, HelperError>
where
	T: FromInputBytes,
{
	let mut stdin = io::stdin();

	let mut buffer = Vec::with_capacity(MAX_READ_BYTES);
	stdin.by_ref().take(MAX_READ_BYTES as u64).read_to_end(&mut buffer)?;

	if buffer.len() == buffer.capacity() {

		let mut extra = [0u8; 1];
		let extra_read = stdin.read(&mut extra)?;
	
		if extra_read != 0 {
			return Err(HelperError::msg("Input too large"));
		}

	}

	T::from_bytes(buffer)
}

