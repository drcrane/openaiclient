#![allow(dead_code)]
#![allow(unused_imports)]

use std::io;
use serde_derive::{Deserialize};
use crate::helpers::{self, EditReplacementLine};
use std::fs;

#[derive(Deserialize)]
pub struct WriteArgs {
	pub path: String,
	pub content: String,
}

#[derive(Deserialize)]
pub struct ReadArgs {
	pub path: String,
	#[serde(default = "ReadArgs::default_offset")]
	pub offset: i32,
	#[serde(default = "ReadArgs::default_limit")]
	pub limit: i32,
}

impl ReadArgs {
	pub fn new() -> Self {
		ReadArgs{ path: "".to_string(), offset: 1, limit: 2000 }
	}
	pub fn default_offset() -> i32 {
		1
	}
	pub fn default_limit() -> i32 {
		2000
	}
}

#[derive(Deserialize)]
pub struct EditArgs {
	pub path: String,
	#[serde(rename = "oldString")]
	pub old_string: String,
	#[serde(rename = "newString")]
	pub new_string: String,
	#[serde(rename = "replaceAll", default = "EditArgs::default_replace_all")]
	pub replace_all: bool,
}

impl EditArgs {
	pub fn default_replace_all() -> bool {
		false
	}
}

pub struct FileTool;

impl FileTool {
	pub fn read(args: ReadArgs) -> Result<String, String> {
		let offset: usize = args.offset.try_into().map_err(|e| "Cannot convert offset into usize".to_string())?;
		let limit: usize = args.limit.try_into().map_err(|e| "Cannot convert limit into usize".to_string())?;
		let file_contents = helpers::read_file(&args.path, offset, limit, 500, true);
		if let Ok(content) = file_contents {
			let result = format!("<path>{}</path>\n<content>\n{}</content>\n", &args.path, &content);
			Ok(result)
		} else {
			file_contents
		}
	}

	fn format_replacements(text: &str, hits: &[EditReplacementLine]) -> String {
		let lines: Vec<_> = text.lines().collect();
		let lines_above = 2;
		let lines_below = 2;

		if hits.is_empty() {
			return "".to_string();
		}

		let mut ranges = Vec::new();

		for line in hits {
			let start = line.start_line.saturating_sub(lines_above);
			let line_count = line.line_count.saturating_sub(1);
			let end = (line.start_line + line_count + lines_below).min(lines.len() - 1);
			ranges.push((start, end));
		}

		// Merge overlapping/adjacent ranges.
		let mut merged = Vec::new();
		let mut current = ranges[0];

		for range in ranges.into_iter().skip(1) {
			if range.0 <= current.1 + 1 {
				current.1 = current.1.max(range.1);
			} else {
				merged.push(current);
				current = range;
			}
		}

		merged.push(current);
		let mut replacements = String::new();
		let mut replacement_count = 0;

		for (start, end) in merged {
			replacements.push_str("<replacement>\n");

			for line_no in start..=end {
				replacements.push_str(&format!("{}: {}\n", line_no + 1, lines[line_no]));
			}

			replacements.push_str("</replacement>");
			replacement_count = replacement_count + 1;
			if replacement_count < hits.len() {
				replacements.push('\n');
			}
		}

		replacements
	}

	pub fn edit(args: EditArgs) -> Result<String, String> {
		// Assuming helpers::edit_file takes path, old_string, new_string, and replace_all
		let edit_result = helpers::edit_file_lines(&args.path, &args.old_string, &args.new_string, args.replace_all);
		match edit_result {
			Ok(updated_content) => {
				let contents = fs::read(&args.path)
					.map_err(|e| format!("failed to read {}: {e}", args.path))?;
				let file_lines = String::from_utf8(contents.to_vec())
					.map_err(|e| format!("failed to convert file contents to utf-8: {e}"))?;
				Ok(format!("File Path: {}\nTotal Replacements: {}\n{}\n",
					&args.path, updated_content.len(), Self::format_replacements(&file_lines, &updated_content)))
			}
			Err(e) => {
				Err(e)
			}
		}
	}

	pub fn write(args: WriteArgs) -> Result<String, String> {
		let write_result = helpers::write_file(&args.path, &args.content);
		match write_result {
			Ok(success) => {
				Ok(format!("File Path: {}\n<result>Success, written {} bytes</result>", &args.path, success.bytes))
			},
			Err(e) => {
				Err(e)
			},
		}
	}
}

