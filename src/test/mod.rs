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

#[derive(Serialize, Deserialize, Debug)]
struct SampleConfig {
	name: String,
}

#[test]
fn load_config() -> Result<(), std::io::Error> {
	let mut config_dir = helpers::config_get_dir(None)?;
	let mut config_file = config_dir.join("openaiclienttest.json");
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

