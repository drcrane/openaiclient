#![allow(unused)]

pub mod helpers;
mod tools;

use std::env;

/*

target/debug/tooltester read '{"path":"README.md","offset":1,"limit":999999999999999999}'

 */

#[tokio::main]
async fn main() -> Result<(), String> {
	let mut dispatcher = tools::Dispatcher{  };

	let args: Vec<String> = env::args().collect();
	println!("{}", &args[1]);
	println!("{}", &args[2]);

	let result = dispatcher.dispatch(&args[1], &args[2]).await?;
	println!("Success:\n{}", result);

	Ok(())
}

