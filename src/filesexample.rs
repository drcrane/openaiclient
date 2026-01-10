#![allow(unused)]

mod tools;

#[tokio::main]
async fn main() -> Result<(), String> {
	let mut dispatcher = tools::Dispatcher{ todoctx: tools::todo::TodoLibrary::new("todolist.sqlite3") };

	//let rt = tokio::runtime::Runtime::new().unwrap();
	//rt.block_on(dispatcher.dispatch("something", "{}"))

	let mut result = dispatcher.dispatch("write", r##"{"path":"test.txt", "content":"# This is a README.md File\n\nThe project is not going very well, I want to find someone to save it.\n\n## Finally, Some Testing\n\nlorem ipsom anyone?\n"}"##).await?;
	println!("Success: {}", result);

	result = dispatcher.dispatch("search_replace", r#"{"file_path":"test.txt", "content":"<<<<<<< SEARCHnot going=======going>>>>>>> REPLACE\n<<<<<<< SEARCH.\n=======!\n>>>>>>> REPLACE"}"#).await?;
	println!("Success: {}", result);

	result = dispatcher.dispatch("read", r#"{"path":"test.txt"}"#).await?;
	println!("Success: {}", result);

	Ok(())
}

