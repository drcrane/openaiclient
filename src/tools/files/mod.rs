#![allow(unused_assignments)]

use std::fs;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use ignore::gitignore::GitignoreBuilder;
use serde_json;
use serde_derive::Deserialize;

#[derive(Deserialize)]
struct SearchReplaceInput {
	file_path: String,
	content: String,
}

#[derive(Debug)]
struct SearchReplaceBlock {
	search: String,
	replace: String,
	raw_block: String,
}

#[derive(Deserialize)]
struct WriteFileArgs {
	path: String,
	content: String,
	#[serde(default)]
	overwrite: bool,
}

pub fn write_file(arguments: &str) -> Result<String, String> {
	let args: WriteFileArgs = serde_json::from_str(arguments)
		.map_err(|e| format!("Invalid JSON arguments: {}", e))?;

	let path = Path::new(&args.path);

	if path.exists() && !args.overwrite {
		return Err(format!(
			"File '{}' already exists. Set overwrite=true to overwrite it.",
			args.path
		));
	}

	if let Some(parent) = path.parent() {
		if !parent.as_os_str().is_empty() {
			fs::create_dir_all(parent)
				.map_err(|e| format!("Failed to create parent directories: {}", e))?;
		}
	}

	let mut file = fs::File::create(path)
		.map_err(|e| format!("Failed to create file '{}': {}", args.path, e))?;

	file.write_all(args.content.as_bytes())
		.map_err(|e| format!("Failed to write file '{}': {}", args.path, e))?;

	let byte_count = args.content.as_bytes().len();

	Ok(format!(
		"Written {} bytes to {}",
		byte_count, args.path
	))
}

pub fn make_gitignore_matcher(gitignore_contents: &str,) -> impl Fn(&Path) -> bool {
	let mut builder = GitignoreBuilder::new("");

	for line in gitignore_contents.lines() {
		builder.add_line(None, line).unwrap();
	}

	let gitignore = builder.build().unwrap();

	move |path: &Path| {
		gitignore
			.matched(path, path.is_dir())
			.is_ignore()
	}
}

pub fn make_gitaccept_matcher(gitignore_contents: &str,) -> impl Fn(&Path) -> Option<PathBuf> {
	let mut builder = GitignoreBuilder::new("");

	for line in gitignore_contents.lines() {
		builder.add_line(None, line).unwrap();
	}

	let gitignore = builder.build().unwrap();

	move |path: &Path| -> Option<PathBuf> {
		if !gitignore
			.matched(path, path.is_dir())
			.is_ignore() {
			Some(PathBuf::from(path))
		} else {
			None
		}
	}
}

#[derive(Deserialize)]
pub struct WriteArgs {
	pub path: String,
	pub content: String,
	#[serde(default)]
	pub append: bool,
}

#[derive(Deserialize)]
pub struct ReadArgs {
	path: String,
	show_line_numbers: Option<bool>,
	offset: Option<usize>,
	limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct EditArgs {
	pub path: String,
	pub old_string: String,
	pub new_string: String,
}

#[derive(Deserialize)]
pub struct MultiEditArgs {
	pub path: String,
	pub edits: Vec<EditOperation>,
}

#[derive(Deserialize)]
pub struct EditOperation {
	pub old_string: String,
	pub new_string: String,
}

pub struct FileLibrary {
}

impl FileLibrary {
	pub fn write_file(args: WriteArgs) -> Result<String, String> {
		if args.append {
			let mut file = OpenOptions::new()
				.append(true)
				.create(true)
				.write(true)
				.open(&args.path)
				.map_err(|e| e.to_string())?;
			
			file.write_all(&args.content.as_bytes())
				.map_err(|e| e.to_string())?;
		} else {
			fs::write(&args.path, &args.content).map_err(|e| e.to_string())?;
		}

		Ok(format!("{} bytes written", args.content.len()))
	}

	pub fn read_file(args: ReadArgs) -> Result<String, String> {
		let content = fs::read_to_string(&args.path).map_err(|e| e.to_string())?;
		let show_line_numbers = args.show_line_numbers.unwrap_or(false);
		let start = args.offset.unwrap_or(1);
		if start == 0 {
			return Err("line_start must be >= 1".into());
		}
		let limit = args.limit.unwrap_or(usize::MAX);

		let lines: Vec<&str> = content.lines().collect();

		let start_idx = start.saturating_sub(1);
		let end_idx = (start + limit).min(lines.len()).min(1000);

		let mut result = String::new();

		for (i, line) in lines[start_idx..end_idx].iter().enumerate() {
			if show_line_numbers {
				result.push_str(&format!("{:>}: {}\n", start_idx + i + 1, line));
			} else {
				result.push_str(line);
				result.push('\n');
			}
		}

		Ok(result)
	}

	pub fn search_replace(json_input: &str) -> Result<String, String> {
		let input: SearchReplaceInput =
			serde_json::from_str(json_input).map_err(|e| e.to_string())?;
	
		let blocks = Self::parse_blocks(&input.content)?;
		Self::apply_blocks(&input.file_path, &blocks, &input.content)
	}
	
	fn parse_blocks(content: &str) -> Result<Vec<SearchReplaceBlock>, String> {
		let mut blocks = Vec::new();
		let mut remaining = content;
	
		loop {
			let start = match remaining.find("<<<<<<< SEARCH") {
				Some(pos) => pos,
				None => break,
			};
	
			let after_start = &remaining[start + "<<<<<<< SEARCH".len()..];
	
			let sep = after_start
				.find("=======")
				.ok_or("Missing ======= separator")?;
	
			let end = after_start
				.find(">>>>>>> REPLACE")
				.ok_or("Missing >>>>>>> REPLACE")?;
	
			let search = after_start[..sep].trim_start_matches('\n').to_string();
			let replace = after_start[sep + "=======".len()..end]
				.trim_start_matches('\n')
				.to_string();
	
			let raw_block = format!(
				"<<<<<<< SEARCH\n{}=======\n{}>>>>>>> REPLACE\n",
				search, replace
			);
	
			blocks.push(SearchReplaceBlock {
				search,
				replace,
				raw_block,
			});
	
			remaining = &after_start[end + ">>>>>>> REPLACE".len()..];
		}
	
		if blocks.is_empty() {
			return Err("No SEARCH/REPLACE blocks found".into());
		}
	
		Ok(blocks)
	}
	
	fn apply_blocks(
		file_path: &str,
		blocks: &[SearchReplaceBlock],
		original_content: &str,
	) -> Result<String, String> {
		let path = Path::new(file_path);
	
		let mut file_content =
			fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
	
		let mut lines_changed = 0;
		let mut warnings = Vec::new();
	
		for block in blocks {
			if !file_content.contains(&block.search) {
				return Err(format!(
					"SEARCH block not found in file:\n{}",
					block.search
				));
			}
	
			let search_lines = block.search.lines().count();
			let replace_lines = block.replace.lines().count();
	
			if search_lines != replace_lines {
				warnings.push(format!(
					"Line count changed from {} to {}",
					search_lines, replace_lines
				));
			}
	
			lines_changed += search_lines;
	
			file_content = file_content.replacen(&block.search, &block.replace, 1);
		}
	
		fs::write(path, &file_content)
			.map_err(|e| format!("Failed to write file: {e}"))?;
	
		// Build report
		let report = format!(
			"file: {}\n\
			 blocks_applied: {}\n\
			 lines_changed: {}\n\
			 content:\n{}\n\
			 warnings: {}\n",
			file_path,
			blocks.len(),
			lines_changed,
			original_content.trim_end(),
			if warnings.is_empty() {
				"none".to_string()
			} else {
				warnings.join("; ")
			}
		);
	
		Ok(report)
	}

	pub fn multiedit(args: MultiEditArgs) -> Result<String, String> {
		let mut content = fs::read_to_string(&args.path).map_err(|e| e.to_string())?;
		
		let original_content = content.clone();
		for edit in &args.edits {
			if let Some(pos) = content.find(&edit.old_string) {
				content.replace_range(pos..pos + edit.old_string.len(), &edit.new_string);
			} else {
				content = original_content.clone();
				return Err(format!("Edit failed: string '{}' not found", edit.old_string));
			}
		}
		
		fs::write(&args.path, &content).map_err(|e| e.to_string())?;
		
		Ok(format!("Applied {} edits successfully", args.edits.len()))
	}

	pub fn edit_file(args: EditArgs) -> Result<String, String> {
		let mut content = fs::read_to_string(&args.path).map_err(|e| e.to_string())?;
		let original = content.clone();
		if let Some(pos) = content.find(&args.old_string) {
			content.replace_range(pos..pos + args.old_string.len(), &args.new_string);
		} else {
			//content = original.clone();
			return Err(format!("Edit failed: string '{}' not found", args.old_string));
		};
		fs::write(&args.path, &content).map_err(|e| e.to_string())?;
		Ok(format!("Edit applied successfully"))
	}
}

