#![allow(unused)]

use clap::{CommandFactory,Parser};
use url::Url;
use std::path::PathBuf;
use std::fs::{File,OpenOptions};
use std::io::{Read,Write};
use std::env;
use serde::ser::StdError;

mod helpers;
mod openaiapi;

#[cfg(test)]
mod test;

#[derive(Parser)]
struct Cli {
	chat_id: String,
	/// The message to send to the assistant (prefix a filename with @ to send that file as your
	/// message)
	message: String,
	#[clap(long, default_value = "user")]
	role: String,
	#[clap(long, default_value = "data")]
	config_dir: PathBuf,
	#[clap(long, default_value = "chats")]
	chats_dir: PathBuf,
	#[clap(long, default_value = "false")]
	write_req_resp: bool,
	#[clap(long)]
	/// function name when role is function
	name: Option<String>,
	#[clap(long)]
	/// tool call id (default is to use the id of the last tool call that does not have a response)
	tool_call_id: Option<String>,
	#[clap(long)]
	pretend: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Cli::parse();

	if args.role == "function" && args.name.is_none() {
		let mut cmd = Cli::command();
		cmd.error(
			clap::error::ErrorKind::ArgumentConflict,
			"When adding a message as a function role a function name is required, see --help",
			).exit();
	}

	let azure_api_key = env::var("AZURE_API_KEY");
	let azure_api_base = env::var("AZURE_API_BASE");
	let azure_api_version = env::var("AZURE_API_VERSION");

	let (api_url, api_key) = if let (Ok(key), Ok(base), Ok(ver)) = (azure_api_key, azure_api_base, azure_api_version) {
		let url_base = format!("{}chat/completions?api-version={}", base, ver);
		(url_base, key)
	} else {
		return Err(Into::<Box<dyn std::error::Error>>::into(std::io::Error::new(std::io::ErrorKind::Other, "Ooops! no environment variables")));
	};

    println!("Got chat_id: {} and message: {}", &args.chat_id, &args.message);

	let mut ctx = openaiapi::ChatContext::new(args.config_dir, args.chats_dir, api_url, api_key)?;
	ctx.write_req_resp = args.write_req_resp;
	ctx.load_or_new_chat(&args.chat_id)?;

	if args.message == "dump" {
		for message in ctx.chat.as_ref().unwrap().messages.iter() {
			if let Some(mesg) = message.content.as_ref() {
				println!("{}", mesg);
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

	let message = match args.message.chars().nth(0).unwrap() {
		'@' => {
			let mut filename = args.message.clone();
			filename.remove(0);
			let mut content = String::new();
			File::open(&filename)?.read_to_string(&mut content)?;
			content
		},
		_ => args.message,
	};

	// Here only one tool call may be added and if more tool calls
	// are pending then the call_api() function will fail and the
	// binary may be called again to add more tool call responses
	// TODO: this needs to be better.

	// If the name is supplied then the response is from a tool
	match args.name {
		Some(name) => ctx.add_tool_message(&args.role, &name, args.tool_call_id.as_deref(), &message),
		None => ctx.add_normal_message(&args.role, &message),
	};

	let response = ctx.call_api().await?;
	//let mut resp_file = OpenOptions::new()
	//	.read(true)
	//	.write(true)
	//	.create(true)
	//	.truncate(true)
	//	.open("response.json")?;
	//writeln!(resp_file, "{}", response)?;
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

