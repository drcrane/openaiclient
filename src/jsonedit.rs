use clap::{Arg, Command};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let matches = Command::new("json-set")
		.version("1.0")
		.about("Sets a value in a JSON file at a specified path")
		.arg(
			Arg::new("file")
				.help("JSON file or '-' for stdin")
				.required(true),
		)
		.arg(
			Arg::new("path")
				.help("Path in JSON (e.g., parent.child[0].key)")
				.required(true),
		)
		.arg(
			Arg::new("value")
				.help("New value to set (as JSON string, number, or literal, or '-' for stdin, or '@filename' for file)")
				.required(true),
		)
		.get_matches();

	let file = matches.get_one::<String>("file").unwrap();
	let path = matches.get_one::<String>("path").unwrap();
	let value_str = matches.get_one::<String>("value").unwrap();

	// Parse the new value
	let new_value: Value = if value_str == "-" {
		// Read from stdin
		let mut buffer = String::new();
		io::stdin().read_to_string(&mut buffer)?;
		serde_json::from_str(&buffer).unwrap_or_else(|_| json!(buffer))
	} else if value_str.starts_with('@') {
		// Read from file
		let filename = &value_str[1..];
		let file_content = fs::read_to_string(filename)?;
		serde_json::from_str(&file_content).unwrap_or_else(|_| json!(file_content))
	} else {
		// Parse as JSON or treat as string
		serde_json::from_str(value_str).unwrap_or_else(|_| json!(value_str))
	};

	// Read JSON from file or stdin
	let json_str = if file == "-" {
		let mut buffer = String::new();
		io::stdin().read_to_string(&mut buffer)?;
		buffer
	} else {
		fs::read_to_string(file)?
	};

	let mut data: Value = serde_json::from_str(&json_str)?;

	// Split path into parts
	let parts: Vec<&str> = path.split('.').collect();
	let mut current = &mut data;

	// Traverse the path
	for part in &parts[..parts.len() - 1] {
		if part.contains('[') {
			// Handle array indices
			let (key, indices) = parse_array_path(part)?;
			if let Value::Object(map) = current {
				current = map.get_mut(key).ok_or_else(|| {
					format!("Key '{}' not found in JSON", key)
				})?;
			}
			for index in indices {
				if let Value::Array(arr) = current {
					if index >= arr.len() {
						return Err(format!("Index {} out of bounds", index).into());
					}
					current = &mut arr[index];
				} else {
					return Err(format!("Expected array at path '{}'", part).into());
				}
			}
		} else {
			// Handle object keys
			let part_string = part.to_string();
			if let Value::Object(map) = current {
				current = map.get_mut(&part_string).ok_or_else(|| {
					format!("Key '{}' not found in JSON", part)
				})?;
			} else {
				return Err(format!("Expected object at path '{}'", part).into());
			}
		}
	}

	// Set the new value
	let last_part = parts.last().unwrap();
	if last_part.contains('[') {
		// Handle array indices in the last part
		let (key, indices) = parse_array_path(last_part)?;
		if let Value::Object(map) = current {
			let arr = map.get_mut(key).ok_or_else(|| {
				format!("Key '{}' not found in JSON", key)
			})?;
			if let Value::Array(arr) = arr {
				if let Some(index) = indices.last() {
					if *index >= arr.len() {
						return Err(format!("Index {} out of bounds", index).into());
					}
					arr[*index] = new_value;
				}
			} else {
				return Err(format!("Expected array at path '{}'", last_part).into());
			}
		}
	} else {
		// Handle object keys in the last part
		if let Value::Object(map) = current {
			map.insert(last_part.to_string(), new_value);
		} else {
			return Err(format!("Expected object at path '{}'", last_part).into());
		}
	}

	// Print the modified JSON
	println!("{}", serde_json::to_string_pretty(&data)?);

	Ok(())
}

// Helper function to parse array indices from path parts
fn parse_array_path(part: &str) -> Result<(&str, Vec<usize>), Box<dyn std::error::Error>> {
	let mut key = part;
	let mut indices = Vec::new();
	if let Some(open_bracket) = part.find('[') {
		key = &part[..open_bracket];
		let rest = &part[open_bracket..];
		for cap in rest.split('[').skip(1) {
			let index_str = cap.split(']').next().unwrap();
			let index = index_str.parse::<usize>()?;
			indices.push(index);
		}
	}
	Ok((key, indices))
}

