use futures_util::StreamExt;
use std::path::{Path,PathBuf};
use serde_json;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use reqwest::header::{CONTENT_TYPE,CONTENT_LENGTH,AUTHORIZATION};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
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
	pub fn tool_request(role: String, content: MessageContent, tool_calls: Vec<ToolCall>) -> Self {
		Message{ role: role, content: Some(content), name: None, tool_calls: Some(tool_calls), tool_call_id: None }
	}
	pub fn tool_response(role: String, name: String, tool_call_id: String, content: MessageContent) -> Self {
		Message{ role: role, name: Some(name), tool_call_id: Some(tool_call_id), content: Some(content), tool_calls: None }
	}
	pub fn human_readable_string(&self) -> String {
		let mut result = String::new();
		result.push_str(&format!("# {}\n", &self.role));
		if let Some(MessageContent::Simple(mesg)) = self.content.as_ref() {
			result.push_str(&format!("{}\n", &mesg));
		}
		if let Some(MessageContent::Multi(parts)) = self.content.as_ref() {
			for part in parts {
				match part {
					ContentPart::Text { text } => result.push_str(&format!("{text}")),
					ContentPart::ImageUrl { image_url } => {
						result.push_str(&format!("Image ({} bytes)", image_url.url.len()))
					}
				}
			}
		}
		if let Some(tool_calls) = self.tool_calls.as_ref() {
			for tool_call in tool_calls.iter() {
				result.push_str(&format!("```{}\n{}\n```", &tool_call.function.name, &tool_call.function.arguments));
			}
		}
		result
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

	// This is actually a tool response, not a tool call request from the model
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
		if let Some(chat) = self.chat.as_mut() {
			chat.stream = Some(true);
		}
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
		let mut stream = req.bytes_stream();
		let mut body = String::with_capacity(4096);
		while let Some(chunk) = stream.next().await {
			let bytes = chunk?;
			let text = String::from_utf8_lossy(&bytes);
			println!("{}", &text);
			body.push_str(&text);
		}
		//let body = req.text().await?;
		if self.write_req_resp {
			fs::write("last_response.json", &body)?;
		}
		// Check if this looks like a streaming response by checking for SSE format
		let response_message = if body.contains("data: ") && body.contains("[DONE]") {
			// This appears to be a streaming response, parse it accordingly
			Self::parse_streaming_response(&body)?
		} else {
			// This is a regular non-streaming response
			Self::parse_response(&body)?
		};
		let result = Ok(Message::human_readable_string(&response_message));
		self.add_message(response_message);
		result
	}

	pub fn parse_streaming_response(accumulated_message: &str) -> Result<Message, Box<dyn std::error::Error>> {
		// Parse the accumulated streaming message to extract the full response
		// Streaming responses come as multiple JSON objects separated by newlines
		let mut full_response = String::new();
		let mut tool_calls = Vec::new();

		let mut tool_call_id = String::new();
		let mut tool_call_type = String::new();
		let mut tool_call_function_name = String::new();
		let mut tool_call_function_arguments = String::new();

		for line in accumulated_message.lines() {
			let line = line.trim();
			if !line.is_empty() && line != "[DONE]" {
				if line.starts_with("data: ") {
					let data = &line[6..]; // Remove "data: " prefix
					if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(data) {
						if let Some(choices) = json_value.get("choices") {
							if let Some(first_choice) = choices.get(0) {
								if let Some(index) = first_choice.get("index") {
									// not really sure what this does.
								}

								if let Some(delta) = first_choice.get("delta") {
									// Handle text content
									if let Some(content) = delta.get("content") {
										if let Some(content_str) = content.as_str() {
											full_response.push_str(content_str);
										}
									}

									// Handle tool calls
									if let Some(tool_calls_array) = delta.get("tool_calls") {
										if let Some(array) = tool_calls_array.as_array() {
											for tool_call_value in array {
												if let Some(id) = tool_call_value.get("id").and_then(serde_json::Value::as_str) {
													tool_call_id = id.to_string();
												}
												if let Some(tool_type) = tool_call_value.get("type").and_then(serde_json::Value::as_str) {
													tool_call_type = tool_type.to_string();
												}
												let function = tool_call_value.get("function").and_then(serde_json::Value::as_object);
												if let Some(value) = function {
													if let Some(name) = value.get("name").and_then(serde_json::Value::as_str) {
														tool_call_function_name = name.to_string();
													}
													if let Some(arguments) = value.get("arguments").and_then(serde_json::Value::as_str) {
														tool_call_function_arguments.push_str(&arguments.to_string());
													}
												}
											}
										}
									}
								}

								if let Some(finish_reason) = first_choice.get("finish_reason").and_then(serde_json::Value::as_str) {
									if finish_reason == "tool_calls" {
										let tool_call: ToolCall = ToolCall{ id: tool_call_id, tool_type: tool_call_type, function: FunctionCall { name: tool_call_function_name, arguments: tool_call_function_arguments } };
										//println!("tool_call: {}", serde_json::to_string(&tool_call)?);
										tool_calls.push(tool_call);
										tool_call_id = String::new();
										tool_call_type = String::new();
										tool_call_function_name = String::new();
										tool_call_function_arguments = String::new();
									}
								}
							}
						}
					}
				}
			}
		}
		// Format the response with tool calls if present
		//if !tool_calls.is_empty() {
		//	let tool_calls_content: Vec<String> = tool_calls.iter().map(|call|
		//		format!("{}: {}", call.function.name, call.function.arguments))
		//		.collect();
		//	full_response.push_str(&format!("\n{}", tool_calls_content.join("\n")));
		//}

		if tool_calls.len() > 1 {
			panic!("Tool calls cannot be more than one!");
		}
		Ok(Message{ role: "assistant".to_string(), content: if full_response.len() > 0 { Some(MessageContent::Simple(full_response)) } else { None }, tool_calls: if !tool_calls.is_empty() { Some(tool_calls) } else { None }, name: None, tool_call_id: None })
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

