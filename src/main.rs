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
	message: Option<String>,
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

	let mut ctx = openaiapi::ChatContext::new(args.config_dir, args.chats_dir, api_url, api_key)?;
	if let (Ok(model_name)) = openaicompat_model_name {
		ctx.set_model_name(&model_name);
	}
	ctx.write_req_resp = args.write_req_resp;
	ctx.load_or_new_chat(&args.chat_id)?;

	if args.dump {
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

	let message = match args.message.as_deref() {
		Some(s) if s.starts_with('@') => {
			// filename
			let mut filename = s.to_string();
			filename.remove(0);
			let mut content = String::new();
			File::open(&filename)?.read_to_string(&mut content)?;
			content
		},
		Some("-") => {
			// stdin
			helpers::read_stdin()?
		},
		Some("") => {
			"".to_string()
		},
		Some(s) => {
			// message supplied on command line
			s.to_string()
		},
		None => {
			// nothing supplied: do not add to the end of the chat log
			"".to_string()
		},
	};

	println!("Got chat_id: {} and message: {}", &args.chat_id, &message);

	// Here only one tool call may be added and if more tool calls
	// are pending then the call_api() function will fail and the
	// binary may be called again to add more tool call responses
	// TODO: this needs to be better.

	// If the name is supplied then the response is from a tool
	let add_tool_res = match args.name {
		Some(name) => ctx.add_tool_message(&args.role, &name, args.tool_call_id.as_deref(), &message),
		None => if message != "" { ctx.add_normal_message(&args.role, &message) } else { Ok(()) },
	};

	if let Err(e) = add_tool_res {
		eprintln!("operation failed {}", e);
		return Err(e);
	}

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

