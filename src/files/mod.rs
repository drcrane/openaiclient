use serde_json;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct WriteArgs {
	pub path: String,
	pub content: String,
}

#[derive(Deserialize)]
pub struct ReadArgs {
	path: String,
	show_line_numbers: Option<bool>,
	line_start: Option<usize>,
	line_count: Option<usize>,
}

use std::fs;
use std::io::{self, Read};

pub struct FileLibrary {
}

impl FileLibrary {
	pub fn write_file(path: &str, content: &str) -> Result<String, String> {
		fs::write(&path, &content).map_err(|e| e.to_string())?;

		Ok(format!("{} bytes written", content.len()))
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
}
