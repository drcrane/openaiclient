#![allow(unused)]

mod tools;

fn main() -> Result<(), String> {
	let mut tasks = tools::Dispatcher{ todoctx: tools::todo::TodoLibrary::new("todolist.sqlite3") };

	let mut result = tasks.dispatch("add_todo_task", r#"{"name":"Work", "task":"Add a function to complete tasks"}"#)?;
	println!("Success: {}", result);

	result = tasks.dispatch("get_todo_lists", "{}")?;
	println!("Success: {}", result);

	result = tasks.dispatch("get_todo_tasks", r#"{"name":"Work"}"#)?;
	println!("Success: {}", result);

	result = tasks.dispatch("complete_todo_task", r#"{"name":"Work", "task":"Add a function to complete tasks"}"#)?;
	println!("Success: {}", result);

	result = tasks.dispatch("delete_todo_task", r#"{"name":"Work", "task":"Add a function to complete tasks"}"#)?;
	println!("Success: {}", result);
	Ok(())
}
