use super::*;
use serde_derive::{Deserialize, Serialize};
use std::fs;

#[test]
fn it_works() {
	let result = 2 + 2;
	assert_eq!(result, 4);
}

#[test]
fn azure_response_parse_test() {
	let mut file = std::fs::File::open("testdata/sampleresponse.json").unwrap();
	let mut content = String::new();
	file.read_to_string(&mut content).unwrap();
	std::mem::drop(file);
	let chat_response = openaiapi::ChatContext::parse_response(&content);
	println!("{:?}", chat_response);
}

#[test]
fn streaming_response_parse_test() {
	let content = std::fs::read_to_string("testdata/streaming_response.txt").unwrap();
	let mut ctx = openaiapi::ChatContext::new(
		std::path::PathBuf::from("."),
		std::path::PathBuf::from("."),
		"http://localhost:8080".to_string(),
		"test_key".to_string()
	).unwrap();
	let result = openaiapi::ChatContext::parse_streaming_response(&content);
	println!("Streaming parse result: {:?}", result);
	assert!(result.is_ok());
	let parsed_content = result.unwrap();
	assert_eq!(parsed_content, "Hello world!");
}

#[test]
fn streaming_response_with_tools_parse_test() {
	let content = std::fs::read_to_string("testdata/streaming_response_with_tools.txt").unwrap();
	let mut ctx = openaiapi::ChatContext::new(
		std::path::PathBuf::from("."),
		std::path::PathBuf::from("."),
		"http://localhost:8080".to_string(),
		"test_key".to_string()
	).unwrap();
	let result = openaiapi::ChatContext::parse_streaming_response(&content);
	println!("Streaming with tools parse result: {:?}", result);
	assert!(result.is_ok());
	let parsed_content = result.unwrap();
	assert!(parsed_content.contains("I need to use a tool to solve this."));
	assert!(parsed_content.contains("test_function: {\"param\":\"value\"}"));
}

#[derive(Serialize, Deserialize, Debug)]
struct SampleConfig {
	name: String,
}

#[test]
fn load_config() -> Result<(), std::io::Error> {
	let mut config_dir = helpers::config_get_dir(Some("openaiclient"))?;
	let mut config_file = config_dir.join("test.json");
	fs::remove_file(config_file);
	let mut config = helpers::config_load::<SampleConfig>("openaiclient", "test");
	println!("-- {:?}", config);
	assert!(config.is_err());
	Ok(())
}

#[test]
fn save_config() {
	let mut config = SampleConfig{ name: "hello".to_string() };
	helpers::config_save("openaiclient", "test", &config);
	println!("{:?}", config);
}

