#![allow(dead_code)]
#![allow(unused_imports)]

use std::io;
use serde_derive::{Deserialize};
use crate::helpers::{self, EditReplacement};

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

	fn format_replacements(replacements: &[EditReplacement]) -> String {
		let mut output = String::new();

		for (i, edit) in replacements.iter().enumerate() {
			output.push_str("<replacement>\n");

			// 1. Reconstruct the entire text block for this edit
			let mut full_text = String::new();
			full_text.push_str(&edit.before_context);
			full_text.push_str(&edit.replacement);
			full_text.push_str(&edit.after_context);

			// 2. Calculate exactly where this combined block starts in the file.
			// We look at how many newlines are in before_context to find the starting line.
			let before_line_count = edit.before_context.lines().count();

			// If before_context doesn't end in a newline, it shares its last line 
			// with the start of the replacement.
			let adjustments = if !edit.before_context.is_empty() && !edit.before_context.ends_with('\n') {
				before_line_count.saturating_sub(1)
			} else {
				before_line_count
			};

			let mut current_line = edit.start_line.saturating_sub(adjustments);

			// 3. Process line by line
			for line in full_text.lines() {
				output.push_str(&format!("{}: {}\n", current_line, line));
				current_line += 1;
			}

			output.push_str("</replacement>");

			if i < replacements.len() - 1 {
				output.push('\n');
			}
		}
		output
	}

	fn format_replacements_no_linenumbers(replacements: &[EditReplacement]) -> String {
		let mut output = String::new();
		for (i, edit) in replacements.iter().enumerate() {
			output.push_str("<replacement>\n");
			if !edit.before_context.is_empty() {
				output.push_str(&edit.before_context);
			}
			output.push_str(&edit.replacement);
			if !edit.after_context.is_empty() {
				output.push_str(&edit.after_context);
				output.push('\n');
			}
			output.push_str("</replacement>");
			// Add a newline between multiple replacements, but not after the last one
			if i < replacements.len() - 1 {
				output.push('\n');
			}
		}
		output
	}

	pub fn edit(args: EditArgs) -> Result<String, String> {
		// Assuming helpers::edit_file takes path, old_string, new_string, and replace_all
		let edit_result = helpers::edit_file(&args.path, &args.old_string, &args.new_string, args.replace_all);
		match edit_result {
			Ok(updated_content) => {
				Ok(format!("<path>{}</path>\n{}\n", &args.path, Self::format_replacements(&updated_content)))
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
				Ok(format!("<path>{}</path>\n<result>Success, written {} bytes</result>", &args.path, success.bytes))
			},
			Err(e) => {
				Err(e)
			},
		}
	}
}

