use serde_json;
use super::files::{FileLibrary, WriteArgs, ReadArgs};
use super::todo::{TodoLibrary, TodoRequest};

pub struct Dispatcher {
	pub todoctx: TodoLibrary,
}

impl Dispatcher {
	pub fn dispatch(&self, function_name: &str, arguments: &str) -> Result<String, String> {
		match function_name {
			"write" => {
				let args: WriteArgs = serde_json::from_str(arguments).map_err(|e| e.to_string())?;
				FileLibrary::write_file(&args.path, &args.content)
			},
			"read" => {
				let args: ReadArgs = serde_json::from_str(arguments).map_err(|e| e.to_string())?;
				FileLibrary::read_file(args)
			},
			"add_todo_task" => {
				let args: TodoRequest = serde_json::from_str(arguments).unwrap_or(TodoRequest { name: None, task: None });
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.todoctx.add_todo_task(&name, &task)
			},
			"complete_todo_task" => {
				let args: TodoRequest = serde_json::from_str(arguments).unwrap_or(TodoRequest { name: None, task: None });
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.todoctx.set_todo_task_complete(&name, &task, true)
			},
			"delete_todo_task" => {
				let args: TodoRequest = serde_json::from_str(arguments).unwrap_or(TodoRequest { name: None, task: None });
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.todoctx.delete_todo_task(&name, &task)
			},
			"get_todo_lists" => {
				self.todoctx.get_todo_lists()
			},
			"get_todo_tasks" => {
				let args: TodoRequest = serde_json::from_str(arguments).unwrap_or(TodoRequest { name: None, task: None });
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				self.todoctx.get_todo_tasks(&name)
			},
			_ => Err(format!("Unknown function: {}", function_name))
		}
	}

}
