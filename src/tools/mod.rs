use serde_json;
//use super::files::{FileLibrary, WriteArgs, ReadArgs, EditArgs, MultiEditArgs};
//use super::todo::{TodoLibrary, TodoRequest};
//pub mod todo;
//pub mod files;
pub mod executor;
use self::executor::{Executor, ExecuteArgs};

pub struct Dispatcher {
}

impl Dispatcher {
	pub async fn dispatch(&mut self, function_name: &str, arguments: &str) -> Result<String, String> {
		match function_name {
			"execute" => {
				let args: ExecuteArgs = serde_json::from_str(arguments).unwrap_or(ExecuteArgs { command: "echo Error".to_string() });
				let result = Executor::execute(args).await?;
				match serde_json::to_string(&result) {
					Ok(r) => {
						Ok(r)
					},
					Err(e) => {
						Err(e.to_string())
					},
				}
			},
			_ => Err(format!("Unknown function: {}", function_name))
		}
	}

}
