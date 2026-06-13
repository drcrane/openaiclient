use serde_json;
//use super::files::{FileLibrary, WriteArgs, ReadArgs, EditArgs, MultiEditArgs};
//use super::todo::{TodoLibrary, TodoRequest};
//pub mod todo;
//pub mod files;
pub mod files;
use self::files::{FileTool, ReadArgs, EditArgs, WriteArgs};
pub mod executor;
use self::executor::{Executor, ExecuteArgs};

pub struct Dispatcher {
}

impl Dispatcher {
	pub async fn dispatch(&mut self, function_name: &str, arguments: &str) -> Result<String, String> {
		match function_name {
			"read" => {
				match serde_json::from_str::<ReadArgs>(arguments) {
					Ok(args) => {
						FileTool::read(args)
					},
					Err(err) => {
						Err(err.to_string())
					},
				}
			},
			"edit" => {
				match serde_json::from_str::<EditArgs>(arguments) {
					Ok(args) => {
						FileTool::edit(args)
					},
					Err(err) => {
						Err(err.to_string())
					},
				}
			},
			"write" => {
				match serde_json::from_str::<WriteArgs>(arguments) {
					Ok(args) => {
						FileTool::write(args)
					},
					Err(err) => {
						Err(err.to_string())
					},
				}
			},
			"execute" => {
				match serde_json::from_str::<ExecuteArgs>(arguments) {
					Ok(args) => {
						let result = Executor::execute(args).await?;
						Ok(serde_json::to_string(&result).map_err(|err| err.to_string())?)
					},
					Err(err) => {
						Err(err.to_string())
					},
				}
			},
			_ => Err(format!("Unknown function: {}", function_name))
		}
	}

}
