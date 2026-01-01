use serde_json;
use serde_derive::Deserialize;

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
	line_start: Option<usize>,
	line_count: Option<usize>,
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

use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

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
		let start = args.line_start.unwrap_or(1);
		if start == 0 {
			return Err("line_start must be >= 1".into());
		}
		let count = args.line_count.unwrap_or(usize::MAX);

		let lines: Vec<&str> = content.lines().collect();

		let start_idx = start.saturating_sub(1);
		let end_idx = (start_idx + count).min(lines.len()).min(1000);

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

	pub fn multiedit(args: MultiEditArgs) -> Result<String, String> {
		let mut content = fs::read_to_string(&args.path).map_err(|e| e.to_string())?;
		
		let mut original_content = content.clone();
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
		let mut original = content.clone();
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
