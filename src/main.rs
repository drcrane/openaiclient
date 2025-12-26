#![allow(unused)]

use clap::{CommandFactory,Parser};
use url::Url;
use std::path::PathBuf;
use std::fs::{File,OpenOptions};
use std::io::{Read,Write};
use std::env;
use serde::ser::StdError;
use base64::{engine::general_purpose, Engine};

mod helpers;
mod openaiapi;
mod todo;

#[cfg(test)]
mod test;

#[derive(Parser)]
struct Cli {
	chat_id: String,
	#[arg(num_args = 0..)]
	/// The message to send to the assistant.\n
	/// Prefix a filename with @ to send that file as your message.
	/// Only text and images (jpg, png) are supported.
	/// Use - to read from stdin (must be the last and only appear once)
	/// stdin must be UTF-8, images are not supported.
	messages: Option<Vec<String>>,
	#[clap(long, default_value = "user")]
	role: String,
	#[clap(long, default_value = "data")]
	config_dir: PathBuf,
	#[clap(long, default_value = "chats")]
	chats_dir: PathBuf,
	#[clap(long, default_value = "false")]
	write_req_resp: bool,
	#[clap(long)]
	/// dump the current chat in a nice format
	dump: bool,
	#[clap(long)]
	/// function name when role is tool
	name: Option<String>,
	#[clap(long)]
	/// tool call id (default is to use the id of the last tool call that does not have a response)
	tool_call_id: Option<String>,
	#[clap(long)]
	pretend: bool,
	#[clap(long, default_value = "false")]
	/// just append the message, do not perform an API call
	no_network: bool,
}

fn make_content_part(message: &str) -> Result<openaiapi::ContentPart, Box<dyn std::error::Error>> {
	let res = if message.starts_with('@') {
		// filename
		let mut filename = message.to_string();
		filename.remove(0);
		let mut content = String::new();
		if filename.ends_with("png") {
			let bytes = std::fs::read(&filename)?;
			content.push_str("data:image/png;base64,");
			content.push_str(&general_purpose::STANDARD.encode(&bytes));
			openaiapi::ContentPart::ImageUrl { image_url: openaiapi::ImageUrlContent { url: content } }
		} else if filename.ends_with("jpg") {
			let bytes = std::fs::read(&filename)?;
			content.push_str("data:image/jpeg;base64,");
			content.push_str(&general_purpose::STANDARD.encode(&bytes));
			openaiapi::ContentPart::ImageUrl { image_url: openaiapi::ImageUrlContent { url: content } }
		} else {
			// assume the file is a text file
			File::open(&filename)?.read_to_string(&mut content)?;
			openaiapi::ContentPart::Text { text: content }
		}
	} else if message == "-" {
		// stdin
		openaiapi::ContentPart::Text { text: helpers::read_stdin()? }
	} else if message == "" {
		openaiapi::ContentPart::Text { text: "".to_string() }
	} else {
		// message supplied on command line
		openaiapi::ContentPart::Text { text: message.to_string() }
	};
	Ok(res)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Cli::parse();

	if args.role == "tool" && args.name.is_none() {
		let mut cmd = Cli::command();
		cmd.error(
			clap::error::ErrorKind::ArgumentConflict,
			"When adding a message as a function role a function name is required, see --help",
			).exit();
	}

	let azure_api_key = env::var("AZURE_API_KEY");
	let azure_api_base = env::var("AZURE_API_BASE");
	let azure_api_version = env::var("AZURE_API_VERSION");

	let openaicompat_api_key = env::var("OAICOMPAT_API_KEY");
	let openaicompat_api_base = env::var("OAICOMPAT_API_BASE");
	let openaicompat_model_name = env::var("OAICOMPAT_MODEL_NAME");

	let (api_url, api_key) = if let (Ok(key), Ok(base), Ok(ver)) = (azure_api_key, azure_api_base, azure_api_version) {
		let url_base = format!("{}chat/completions?api-version={}", base, ver);
		(url_base, key)
	} else if let (Ok(key), Ok(base)) = (openaicompat_api_key, openaicompat_api_base) {
		let url_base = format!("{}/chat/completions", base);
		(url_base, key)
	} else {
		return Err(Into::<Box<dyn std::error::Error>>::into(std::io::Error::new(std::io::ErrorKind::Other, "Ooops! no environment variables")));
	};

	//let message = match args.message {
	//	Some(msg) => msg,
	//	None => "".to_string(),
	//};
	
	if let Some(messages) = args.messages.as_ref() {
		let mut count = 0;
		for message in messages {
			if message == "-" {
				count = count + 1;
				if (count > 1) {
					return Err(Into::<Box<dyn std::error::Error>>::into(std::io::Error::new(std::io::ErrorKind::Other, "Cannot have more than one stdin/- argument!")));
				}
			}
		}
		if count > 0 {
			if let Some(message) = messages.last() {
				if (message != "-") {
					return Err(Into::<Box<dyn std::error::Error>>::into(std::io::Error::new(std::io::ErrorKind::Other, "Stdin must be the last positional argument")));
				}
			}
		}
	}

	let mut ctx = openaiapi::ChatContext::new(args.config_dir, args.chats_dir, api_url, api_key)?;
	if let (Ok(model_name)) = openaicompat_model_name {
		ctx.set_model_name(&model_name);
	}
	ctx.write_req_resp = args.write_req_resp;
	ctx.load_or_new_chat(&args.chat_id)?;

	if args.dump {
		for message in ctx.chat.as_ref().unwrap().messages.iter() {
		//for message in &ctx.chat.as_ref().unwrap().messages {
			if let Some(content) = message.content.as_ref() {
				match content {
					openaiapi::MessageContent::Simple(txt) => println!("Text: {txt}"),
					openaiapi::MessageContent::Multi(parts) => println!("Multi"),
				}
			}
			if let Some(openaiapi::MessageContent::Simple(mesg)) = message.content.as_ref() {
				println!("{}", mesg);
			}
			if let Some(openaiapi::MessageContent::Multi(parts)) = message.content.as_ref() {
				for part in parts {
					match part {
						openaiapi::ContentPart::Text { text } => println!("Text: {text}"),
						openaiapi::ContentPart::ImageUrl { image_url } => {
							println!("Image ({} bytes)", image_url.url.len())
						}
					}
				}
			}
			if let Some(tool_calls) = message.tool_calls.as_ref() {
				for tool_call in tool_calls.iter() {
					println!("```{}", &tool_call.function.name);
					println!("{}", &tool_call.function.arguments);
					println!("```");
				}
			}
		}
		return Ok(());
	}

	let content = if let Some(messages) = args.messages.as_ref() {
		let mut content_parts: Vec<openaiapi::ContentPart> = Vec::new();
		for message in messages {
			let part = make_content_part(message)?;
			if let openaiapi::ContentPart::Text { text } = &part {
				println!("Text: {text}");
			} else
			if let openaiapi::ContentPart::ImageUrl { image_url } = &part {
				println!("Image ({} bytes)", image_url.url.len());
			}
			content_parts.push(part);
		}
		if content_parts.len() == 1 {
			if let openaiapi::ContentPart::Text { text } = &content_parts[0] {
				openaiapi::MessageContent::Simple(text.to_string())
			} else {
				openaiapi::MessageContent::Multi(content_parts)
			}
		} else {
			openaiapi::MessageContent::Multi(content_parts)
		}
	} else {
		openaiapi::MessageContent::Simple("".to_string())
	};


	//println!("Got chat_id: {} and message: {}", &args.chat_id, &message);
	println!("Got chat_id: {}", &args.chat_id);

	// Here only one tool call may be added and if more tool calls
	// are pending then the call_api() function will fail and the
	// binary may be called again to add more tool call responses
	// TODO: this needs to be better.

	// If the name is supplied then the response is from a tool
	let add_tool_res = match args.name {
		Some(name) => {
			ctx.add_tool_message(&args.role, &name, args.tool_call_id.as_deref(), content)
		},
		None => if let openaiapi::MessageContent::Simple(text) = &content {
			if (text != "") {
				ctx.add_normal_message(&args.role, content)
			} else {
				Ok(())
			}
		} else if let openaiapi::MessageContent::Multi(elements) = &content {
			ctx.add_normal_message(&args.role, content)
		} else {
			Ok(())
		},
	};

	//if let Err(e) = add_tool_res {
	//	eprintln!("operation failed {}", e);
	//	return Err(e);
	//}

	let response = if args.no_network { "No network".to_string() } else { ctx.call_api().await? };
	ctx.save_chat()?;
	println!("{}", response);
	Ok(())
}

async fn get_information(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
	let url = Url::parse(url_str)?;
	let body = reqwest::get(url).await?.text().await?;
	println!("body = {:?}", body);
	Ok(())
}

