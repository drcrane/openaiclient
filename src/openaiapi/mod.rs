use std::path::{Path,PathBuf};
use serde_json;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use reqwest::header::{CONTENT_TYPE,CONTENT_LENGTH};
use std::fs;
//use std::rc::Rc;

use crate::helpers;

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCall {
	pub name: String,
	pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
	pub role: String,
	pub content: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub function_call: Option<FunctionCall>,
}

impl Message {
	pub fn normal(role: String, content: String) -> Self {
		Message{ role: role, content: Some(content), name: None, function_call: None }
	}
	pub fn function_response(role: String, name: String, content: String) -> Self {
		Message{ role: role, content: Some(content), name: Some(name), function_call: None }
	}
}

#[derive(Serialize, Deserialize)]
pub struct FunctionProperty {
	#[serde(rename="type")]
	property_type: String,
	description: String,
	#[serde(rename="enum", skip_serializing_if = "Option::is_none")]
	accepted_values_enum: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionParameters {
	#[serde(rename="type")]
	parameter_type: String,
	properties: HashMap<String, FunctionProperty>,
}

#[derive(Serialize, Deserialize)]
pub struct Function {
	name: String,
	description: String,
	parameters: FunctionParameters,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Chat {
	model: String,
	pub messages: Vec<Message>,
	#[serde(skip_serializing_if="Option::is_none")]
	functions: Option<Vec<Function>>,
	max_tokens: u32,
	temperature: f64,
	frequency_penalty: u32,
	presence_penalty: u32,
	top_p: f64,
	stop: Option<Vec<String>>,
}

pub struct ChatContext {
	pub chat: Option<Chat>,
	chat_id: Option<String>,
	config_dir: PathBuf,
	chats_dir: PathBuf,
	api_key: String,
	post_url: url::Url,
	dirty: bool,
	pub write_req_resp: bool,
}

impl ChatContext {
	pub fn new(config_dir: PathBuf, chats_dir: PathBuf, post_url: String, api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
		Ok(ChatContext {
			chat: None,
			chat_id: None,
			config_dir: config_dir,
			chats_dir: chats_dir,
			api_key: api_key,
			post_url: url::Url::parse(&post_url)?,
			dirty: true,
			write_req_resp: false,
		})
	}

	pub fn new_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
		let mut empty_chat_file: PathBuf = self.config_dir.clone();
		empty_chat_file.push("empty_chat.json");
		let empty_chat = helpers::read_from_json::<Chat>(empty_chat_file)?;
		self.chat = Some(empty_chat);
		self.chat_id = Some(chat_id.to_string());
		// if the chats_dir is not found then an error will be sent from this line (the ? operator)
		let md = fs::metadata(&self.chats_dir)?;
		if md.permissions().readonly() {
			Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Cannot write to chats_dir")))
		} else {
			Ok(())
		}
	}

	pub fn save_chat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
		if self.dirty {
			let mut chat_file: PathBuf = self.chats_dir.clone();
			if let (Some(chat_id), Some(chat)) = (&self.chat_id, &self.chat) {
				chat_file.push(chat_id.to_string() + ".json");
				Ok(helpers::save_to_json(chat_file, chat)?)
			} else {
				Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No chat id")))
			}
		} else {
			Ok(())
		}
	}

	pub fn load_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
		let mut chat_file: PathBuf = self.chats_dir.clone();
		chat_file.push(chat_id.to_string() + ".json");
		match helpers::read_from_json::<Chat>(chat_file) {
			Ok(chat) => {
				self.chat = Some(chat);
				self.chat_id = Some(chat_id.to_string());
				self.dirty = false;
				Ok(())
			},
			Err(err) => { return Err(Box::new(err)); },
		}
	}

	pub fn load_or_new_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
		if (self.load_chat(&chat_id).is_ok()) {
			Ok(())
		} else {
			self.new_chat(&chat_id)
		}
	}

	pub fn current_chat(&mut self) -> Result<&mut Chat, Box<dyn std::error::Error>> {
		match self.chat.as_mut() {
			Some(chat) => Ok(chat),
			None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No chat currently loaded"))),
		}
	}

	pub fn add_message(&mut self, role: &str, name: Option<String>, message: &str) -> Result<(), Box<dyn std::error::Error>> {
		match name {
			Some(name) => {
				self.current_chat()?.messages.push(Message::function_response(role.to_string(), name, message.to_string()));
			},
			None => {
				self.current_chat()?.messages.push(Message::normal(role.to_string(), message.to_string()));
			},
		};
		self.dirty = true;
		Ok(())
	}

	pub async fn call_api(&mut self) -> Result<String, Box<dyn std::error::Error>> {
		let serialised = serde_json::to_string_pretty(&self.chat)?;
		if self.write_req_resp {
			fs::write("last_request.json", &serialised)?;
		}
		let url = self.post_url.clone();
		let client = reqwest::Client::new();
		let req = client
			.post(url)
			.header("api-key", &self.api_key)
			.header(CONTENT_TYPE, "application/json")
			.body(serialised)
			.send()
			.await?;
		let body = req.text().await?;
		if self.write_req_resp {
			fs::write("last_response.json", &body)?;
		}
		let response = Self::parse_response(&body)?;
		let content = match response.content.as_ref() {
			Some(content) => content.to_string(),
			None => "".to_string(),
		};
		self.chat.as_mut().ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Chat not present in context")))?.messages.push(response);
		Ok(content)
	}

	pub fn parse_response(response: &str) -> Result<Message, Box<dyn std::error::Error>> {
		let mut json: serde_json::Value = serde_json::from_str(&response)?;
		let mut message = if let Some(mut mesg) = json
				.get_mut("choices").ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No choices in the return object")))?
				.get_mut(0).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No element 0 in the choices object")))?
				.get_mut("message") {
			mesg.take()
		} else {
			return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No message in the choices element 0")));
		};
		let res: Message = serde_json::from_value(message)?;
		Ok(res)
	}
}

