use std::fs;
use std::path::Path;
use std::io::Write;
use serde_derive::Deserialize;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;

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

pub fn search_replace(json_input: &str) -> Result<String, String> {
	let input: SearchReplaceInput =
		serde_json::from_str(json_input).map_err(|e| e.to_string())?;

	let blocks = parse_blocks(&input.content)?;
	apply_blocks(&input.file_path, &blocks, &input.content)
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

/// Apply blocks and return a report string
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

pub fn make_gitaccept_matcher(gitignore_contents: &str,) -> impl Fn(&Path) -> bool {
	let mut builder = GitignoreBuilder::new("");

	for line in gitignore_contents.lines() {
		builder.add_line(None, line).unwrap();
	}

	let gitignore = builder.build().unwrap();

	move |path: &Path| {
		!gitignore
			.matched(path, path.is_dir())
			.is_ignore()
	}
}
