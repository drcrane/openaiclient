#![allow(dead_code)]
#![allow(unused_imports)]

use std::io;
use serde_derive::{Deserialize};
use crate::helpers::{self};

#[derive(Deserialize)]
pub struct WriteArgs {
	pub path: String,
	pub content: String,
}

#[derive(Deserialize)]
pub struct ReadArgs {
	pub path: String,
	pub offset: i32,
	pub limit: i32,
}

impl ReadArgs {
	pub fn new() -> Self {
		ReadArgs{ path: "".to_string(), offset: 1, limit: 2000 }
	}
}

#[derive(Deserialize)]
pub struct EditArgs {
	pub path: String,
	#[serde(rename = "oldString")]
	pub old_string: String,
	#[serde(rename = "newString")]
	pub new_string: String,
	#[serde(rename = "replaceAll")]
	pub replace_all: bool,
}

pub struct FileTool;

impl FileTool {
	pub fn read(args: ReadArgs) -> Result<String, String> {
		let file_contents = helpers::read_file(&args.path, args.offset.try_into().unwrap_or(1), args.limit.try_into().unwrap_or(2000), 500, true);
		if let Ok(content) = file_contents {
			let result = format!("<path>{}</path>\n<content>\n{}</content>\n", &args.path, &content);
		} else {
			//
		}
		Ok("<path></path>\n<content>\n</content>\n".to_string())
	}
}

