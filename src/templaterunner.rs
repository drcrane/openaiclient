#![allow(unused)]

mod helpers;

use std::env;
use std::fs;
use helpers::{ TemplateProcessor, read_file };

// execution:
// templaterunner <TemplateFile> <KeyInTemplateFileToReplace> <FileToReplaceKeyWith>
// templaterunner testdata/TEMPLATE.md REPLACED_CONTENT testdata/TEMPLATE_CONTENT.md

#[tokio::main]
async fn main() -> Result<(), String> {
	let args: Vec<String> = env::args().collect();
	let content = read_file(&args[3], 1, 1000, 79, true)?;
	let key = args[2].to_string();
	let template = fs::read_to_string(&args[1].to_string()).map_err(|e| e.to_string())?;
	let mut tmpl = TemplateProcessor::new();
	tmpl.add_replacement(key, content);
	let result = tmpl.process_template(&template);
	print!("{}", &result);
	Ok(())
}


