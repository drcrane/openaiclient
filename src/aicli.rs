use std::fs;
use std::env;
use std::io;

mod helpers;
use helpers::TemplateProcessor;

fn main() -> io::Result<()> {
	let template = fs::read_to_string("data/SYSTEM_PROMPT.md")?;
	
	let mut processor = TemplateProcessor::new();
	if let Ok(pwd) = env::var("PWD") {
		processor.add_replacement("PWD".to_string(), pwd);
	}

	let output = processor.process_template(&template);
	println!("{}", output);
	Ok(())
}

