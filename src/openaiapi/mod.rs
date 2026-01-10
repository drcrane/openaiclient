use std::path::{Path,PathBuf};
use serde_json;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use reqwest::header::{CONTENT_TYPE,CONTENT_LENGTH,AUTHORIZATION};
use std::fs;
use thiserror::Error;
//use std::rc::Rc;

use crate::helpers;

#[derive(Debug)]
pub enum ChatErrorKind {
	ChatContainsNoMessages,
	SystemPromptNotFound,
	LastToolCallIdNotFound,
	LastMessageFromAssistant,
	Other,
}

#[derive(Debug)]
pub struct ChatError {
	pub kind: ChatErrorKind,
	pub message: String,
}

impl std::error::Error for ChatError {
}

impl ChatError {
	pub fn new(chat_error_kind: ChatErrorKind, message: &str) -> Self {
		ChatError{kind: chat_error_kind, message: message.to_string()}
	}
}

impl From<url::ParseError> for ChatError {
	fn from(e: url::ParseError) -> Self {
		ChatError{kind: ChatErrorKind::Other, message: String::from("URL Parse Error")}
	}
}

impl From<std::io::Error> for ChatError {
	fn from(e: std::io::Error) -> Self {
		ChatError{kind: ChatErrorKind::Other, message: String::from("IO Error")}
	}
}

impl std::fmt::Display for ChatError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "ChatError Occured {}", self.message)
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCall {
	pub name: String,
	pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
	pub id: String,
	#[serde(rename = "type")]
	pub tool_type: String,
	pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
	pub role: String,
	pub content: Option<MessageContent>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tool_call_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
	pub fn normal(role: String, content: MessageContent) -> Self {
		Message{ role: role, content: Some(content), name: None, tool_calls: None, tool_call_id: None }
	}
	pub fn tool_response(role: String, name: String, tool_call_id: String, content: MessageContent) -> Self {
		Message{ role: role, name: Some(name), tool_call_id: Some(tool_call_id), content: Some(content), tool_calls: None }
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
	Simple(String),
	Multi(Vec<ContentPart>),
}

impl From<&str> for MessageContent {
	fn from(s: &str) -> Self {
		MessageContent::Simple(s.to_string())
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")] //, rename_all = "lowercase")]
pub enum ContentPart {
	#[serde(rename = "text")]
	Text {
		text: String,
	},
	#[serde(rename = "image_url")]
	ImageUrl {
		image_url: ImageUrlContent,
	},
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUrlContent {
	pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionProperty {
	#[serde(rename = "type")]
	property_type: String,
	description: String,
	#[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
	accepted_values_enum: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionParameters {
	#[serde(rename = "type")]
	parameter_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	properties: Option<HashMap<String, FunctionProperty>>,
}

#[derive(Serialize, Deserialize)]
pub struct Function {
	name: String,
	description: String,
	parameters: FunctionParameters,
}

#[derive(Serialize, Deserialize)]
pub struct Tool {
	#[serde(rename = "type")]
	tool_type: String,
	function: Function,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Chat {
	model: String,
	pub messages: Vec<Message>,
	#[serde(skip_serializing_if = "Option::is_none")]
	tools: Option<Vec<Tool>>,
	max_tokens: u32,
	temperature: f64,
	frequency_penalty: u32,
	presence_penalty: u32,
	top_p: f64,
	stop: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	stream: Option<bool>,
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
	model_name: Option<String>,
}

impl ChatContext {
	pub fn new(config_dir: PathBuf, chats_dir: PathBuf, post_url: String, api_key: String) -> Result<Self, ChatError> {
		Ok(ChatContext {
			chat: None,
			chat_id: None,
			config_dir: config_dir,
			chats_dir: chats_dir,
			api_key: api_key,
			post_url: url::Url::parse(&post_url)?,
			dirty: true,
			write_req_resp: false,
			model_name: None,
		})
	}

	pub fn set_model_name(&mut self, model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
		self.model_name = Some(model_name.to_string());
		Ok(())
	}

	pub fn set_system_prompt(&mut self, system_prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
		if let Some(message) = self.chat.as_mut().and_then(|chat| chat.messages.first_mut()) {
			if (message.role != "system") {
				Err(Box::new(ChatError::new(ChatErrorKind::SystemPromptNotFound, "First message was not a system prompt")))
			} else {
				message.content = Some(MessageContent::from(system_prompt));
				self.dirty = true;
				Ok(())
			}
		} else {
			Err(Box::new(ChatError::new(ChatErrorKind::SystemPromptNotFound, "There are no messages.")))
		}
	}

	pub fn new_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
		let mut empty_chat_file: PathBuf = self.config_dir.clone();
		empty_chat_file.push("empty_chat.json");
		//println!("Loading template from: {}", empty_chat_file.display());
		let mut empty_chat = helpers::read_from_json::<Chat>(empty_chat_file)?;
		if empty_chat.model == "" {
			if let Some(model) = &self.model_name {
				empty_chat.model = model.to_string();
			}
		}
		self.chat = Some(empty_chat);
		self.chat_id = Some(chat_id.to_string());
		let serialised = serde_json::to_string_pretty(&self.chat)?;
		println!("Serialised Chat: {}", serialised);
		// if the chats_dir is not found then an error will be sent from this line (the ? operator)
		let md = fs::metadata(&self.chats_dir)?;
		if md.permissions().readonly() {
			Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Cannot write to chats_dir")))
		} else {
			//Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Just stop executing here")))
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

	pub fn get_last_message(&self) -> Result<&Message, Box<dyn std::error::Error>> {
		match self.chat.as_ref() {
			Some(chat) => {
				if let Some(message) = chat.messages.last() {
					Ok(&message)
				} else {
					Err(Box::new(ChatError::new(ChatErrorKind::ChatContainsNoMessages, "No messages in loaded chat")))
				}
			},
			None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No chat currently loaded"))),
		}
	}

	pub fn get_last_pending_tool_call_id(&self) -> Result<Option<String>, ChatError> {
		let mut tool_call_id: Option<String> = None;
		let mut tool_call_ids: Vec<String> = Vec::new();
		for message in &self.chat.as_ref().ok_or(ChatError::new(ChatErrorKind::ChatContainsNoMessages, "No Messages"))?.messages {
			if Option::is_some(&message.tool_calls) {
				let tool_calls = message.tool_calls.as_ref();
				for tool_call in tool_calls.ok_or(ChatError::new(ChatErrorKind::Other, "No Tool Calls"))? {
					tool_call_ids.push(tool_call.id.clone());
				}
			}
			if Option::is_some(&message.tool_call_id) {
				tool_call_ids.retain(|s| s != message.tool_call_id.as_ref().unwrap());
			}
		}
		if let Some(last_tool_call_id) = tool_call_ids.first() {
			Ok(Some(last_tool_call_id.to_string()))
		} else {
			Ok(None)
			//Err(ChatError::new(ChatErrorKind::LastToolCallIdNotFound, "No Last Tool Call ID Found"))
		}
	}

	pub fn get_tool_call(&self, tool_call_id: &str) -> Result<&ToolCall, ChatError> {
		let messages = &self.chat.as_ref().ok_or(ChatError::new(ChatErrorKind::ChatContainsNoMessages, "No Messages"))?.messages;
		for message in messages {
			if let Some(tool_calls) = &message.tool_calls {
				for tool_call in tool_calls {
					if tool_call.id == tool_call_id {
						return Ok(tool_call);
					}
				}
			}
		}
		Err(ChatError::new(ChatErrorKind::LastToolCallIdNotFound, tool_call_id))
	}

	pub fn add_message(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
		self.current_chat()?.messages.push(message);
		self.dirty = true;
		Ok(())
	}

	pub fn add_normal_message(&mut self, role: &str, message: MessageContent) -> Result<(), Box<dyn std::error::Error>> {
		let messages = &mut self.current_chat()?.messages;
		if let Some(last_message) = messages.last_mut() {
			if (last_message.role == role) {
				return Err(Box::new(ChatError::new(ChatErrorKind::Other, "Last message was from same role")));
			}
		}
		//messages.push(Message::normal(role.to_string(), message.into()));
		messages.push(Message::normal(role.to_string(), message));
		self.dirty = true;
		Ok(())
	}

	pub fn add_tool_message(&mut self, role: &str, name: &str, tool_call_id: &str, message: MessageContent) -> Result<(), Box<dyn std::error::Error>> {
		let message = Message{
			role: role.to_string(),
			name: Some(name.to_string()),
			tool_call_id: Some(tool_call_id.to_string()),
			content: Some(message),
			tool_calls: None,
		};
		self.current_chat()?.messages.push(message);
		self.dirty = true;
		Ok(())
	}

	pub async fn call_api(&mut self) -> Result<String, Box<dyn std::error::Error>> {
		let serialised = serde_json::to_string_pretty(&self.chat)?;
		if self.write_req_resp {
			fs::write("last_request.json", &serialised)?;
		}
		if self.get_last_message()?.role == "assistant" {
			return Err(Box::new(ChatError::new(ChatErrorKind::LastMessageFromAssistant, "Last message was from the assistant")));
		}
		if let Err(err) = self.get_last_pending_tool_call_id() {
			if ! matches!(err.kind, ChatErrorKind::LastToolCallIdNotFound) {
				return Err(Box::new(err));
			}
		}
		let url = self.post_url.clone();
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(60 * 10))
			.build()?;
		let authorization = format!("Bearer {}", self.api_key);
		let req = client
			.post(url)
			.header("api-key", &self.api_key)
			.header(CONTENT_TYPE, "application/json")
			.header(AUTHORIZATION, authorization)
			.body(serialised)
			.send()
			.await?;
		let body = req.text().await?;
		if self.write_req_resp {
			fs::write("last_response.json", &body)?;
		}
		let response = Self::parse_response(&body)?;
		let human_content = match response.content.as_ref() {
			Some(MessageContent::Simple(content)) => content.to_string(),
			Some(MessageContent::Multi(content_parts)) => {
				let mut s = String::new();
				for part in content_parts {
					if let ContentPart::Text { text } = &part {
						s = s + text + "\n";
					} else
					if let ContentPart::ImageUrl { image_url } = &part {
						s.push_str(&format!("Image ({} bytes)\n", image_url.url.len()));
					}
				}
				s
			},
			None => "".to_string(),
		};
		//serde_json::to_string_pretty(&tool_calls)?,
		let content = match response.tool_calls.as_ref() {
			Some(tool_calls) => {
				let formatted_calls: Vec<String> = tool_calls.into_iter().map(|call|
					format!("{}: {}\n", call.function.name, call.function.arguments))
					.collect();
				format!("{}\n{}", &human_content, &formatted_calls.join("\n"))
			},
			None => human_content,
		};
		// in the case of a tool call...
		//self.chat.as_mut().ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Chat not present in context")))?.messages.push(response);
		self.add_message(response);
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

