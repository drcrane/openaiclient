#![allow(unused)]

mod tools;

fn main() -> Result<(), String> {
	let mut dispatcher = tools::Dispatcher{ todoctx: tools::todo::TodoLibrary::new("todolist.sqlite3") };

	let mut result = dispatcher.dispatch("write", r#"{"path":"test.txt", "content":"Some Testing\n"}"#)?;
	println!("Success: {}", result);

	result = dispatcher.dispatch("read", r#"{"path":"test.txt"}"#)?;
	println!("Success: {}", result);

	Ok(())
}

