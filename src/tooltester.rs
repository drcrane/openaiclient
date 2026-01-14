#![allow(unused)]

mod tools;

use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
	let mut dispatcher = tools::Dispatcher{ todoctx: tools::todo::TodoLibrary::new("todolist.sqlite3") };

	let args: Vec<String> = env::args().collect();
	println!("{}", &args[1]);
	println!("{}", &args[2]);

	let result = dispatcher.dispatch(&args[1], &args[2]).await?;
	println!("Success:\n{}", result);

	Ok(())
}

