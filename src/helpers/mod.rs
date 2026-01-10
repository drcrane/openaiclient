#![allow(dead_code)]
#![allow(unused_imports)]

//use std::fs;
use std::fs::{self, File, OpenOptions};
//use std::io;
use std::io::{self, Read, Write, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::env;
use std::process;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::string;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
	Normal,
	PossibleOpen,
	InKey,
	PossibleClose,
}

pub struct TemplateProcessor {
	replacements: HashMap<String, String>,
}

impl TemplateProcessor {
	pub fn new() -> Self {
		Self {
			replacements: HashMap::new(),
		}
	}

	pub fn with_replacements(replacements: HashMap<String, String>) -> Self {
		Self { replacements }
	}

	pub fn add_replacement(&mut self, key: String, value: String) {
		self.replacements.insert(key, value);
	}

	pub fn remove_replacement(&mut self, key: &str) -> Option<String> {
		self.replacements.remove(key)
	}

	pub fn get_replacement(&self, key: &str) -> Option<&String> {
		self.replacements.get(key)
	}

	pub fn replacements(&self) -> &HashMap<String, String> {
		&self.replacements
	}

	pub fn process_template(&self, template: &str) -> String {
		let mut output = String::new();
		let mut state = State::Normal;
		let mut current_key = String::new();
		
		for ch in template.chars() {
			match state {
				State::Normal => {
					if ch == '{' {
						state = State::PossibleOpen;
					} else {
						output.push(ch);
					}
				}
				State::PossibleOpen => {
					if ch == '%' {
						state = State::InKey;
						current_key.clear();
					} else {
						output.push('{');
						output.push(ch);
						state = State::Normal;
					}
				}
				State::InKey => {
					if ch == '%' {
						state = State::PossibleClose;
					} else {
						current_key.push(ch);
					}
				}
				State::PossibleClose => {
					if ch == '}' {
						// Found complete tag: {% key %}
						let key = current_key.trim();
						if let Some(replacement) = self.replacements.get(key) {
							output.push_str(replacement);
						} else {
							// Key not found, output original tag
							output.push_str("{%");
							output.push_str(key);
							output.push_str("%}");
						}
						state = State::Normal;
					} else {
						// Not a closing brace, so the '%' was part of the key
						current_key.push('%');
						current_key.push(ch);
						state = State::InKey;
					}
				}
			}
		}
		
		match state {
			State::PossibleOpen => {
				output.push('{');
			}
			State::InKey => {
				output.push_str("{%");
				output.push_str(&current_key);
			}
			State::PossibleClose => {
				output.push_str("{%");
				output.push_str(&current_key);
				output.push('%');
			}
			State::Normal => {}
		}
		
		output
	}
}

/// Move a file from `src` to `dst`.
/// - First tries `fs::rename` (fast, atomic on same filesystem).
/// - If that fails (commonly cross-filesystem), falls back to copying then removing the source.
/// - Tries to preserve file permissions where possible.
///
/// Returns `Ok(())` on success or the last encountered `io::Error`.
pub fn move_file_fallback(src: &Path, dst: &Path) -> io::Result<()> {
    // Fast path: try rename first
    match fs::rename(src, dst) {
        Ok(()) => return Ok(()),
        Err(_) => {
            // If rename failed for some reason other than cross-device, we still try fallback.
            // We'll keep the rename error only if fallback also fails, to surface the most relevant error.
            // Proceed to copy+remove fallback below.
            let copy_result = copy_with_permissions(src, dst);
            match copy_result {
                Ok(()) => {
                    // Remove source only after successful copy
                    if let Err(remove_err) = fs::remove_file(src) {
                        // Attempt to clean up the partially-copied dst on failure to remove source.
                        let _ = fs::remove_file(dst);
                        return Err(remove_err);
                    }
                    return Ok(());
                }
                Err(copy_err) => {
                    // Return the copy error; if you prefer the original rename error instead, return rename_err.
                    return Err(copy_err);
                }
            }
        }
    }
}

fn copy_with_permissions(src: &Path, dst: &Path) -> io::Result<()> {
    // Ensure parent directory of dst exists (or let copy fail)
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Perform copy (streams content)
    fs::copy(src, dst)?;

    // Try to preserve permissions; ignore if not supported on platform
    if let Ok(metadata) = fs::metadata(src) {
        let perm = metadata.permissions();
        let _ = fs::set_permissions(dst, perm);
    }

    Ok(())
}

