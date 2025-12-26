#![allow(unused)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(invalid_value)]
use std::ffi::{CStr, CString};
use std::ptr;
use core::mem;
use libc::{c_char, c_int};

#[repr(C)]
pub struct Sqlite3 { _unused: [u8; 0] }

#[repr(C)]
pub struct Sqlite3Stmt { _unused: [u8; 0] }

pub type sqlite3_destructor_type = unsafe extern "C" fn(arg1: *mut ::core::ffi::c_void);
pub fn SQLITE_STATIC() -> sqlite3_destructor_type {
	unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(0_isize) }
}
pub fn SQLITE_TRANSIENT() -> sqlite3_destructor_type {
	unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(-1_isize) }
}

const SQLITE_DONE: c_int = 101;
const SQLITE_ROW: c_int = 100;

//pub type sqlite3_destructor_type = ::core::option::Option<unsafe extern "C" fn(arg1: *mut ::core::ffi::c_void)>;
//pub fn SQLITE_STATIC() -> sqlite3_destructor_type {
//	Some(unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(0_isize) })
//}
//pub fn SQLITE_TRANSIENT() -> sqlite3_destructor_type {
//	Some(unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(-1_isize) })
//}

//const SQLITE_STATIC: sqlite3_destructor_type = unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(0_isize) };
//const SQLITE_TRANSIENT: sqlite3_destructor_type = unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(-1_isize) };

#[link(name = "sqlite3")]
extern "C" {
fn sqlite3_open(filename: *const c_char, ppDb: *mut *mut Sqlite3) -> c_int;
fn sqlite3_exec(db: *mut Sqlite3, sql: *const c_char, callback: Option<extern "C" fn() -> c_int>, arg: *mut std::ffi::c_void, errmsg: *mut *mut c_char) -> c_int;
fn sqlite3_prepare_v2(db: *mut Sqlite3, sql: *const c_char, n_byte: c_int, ppStmt: *mut *mut Sqlite3Stmt, tail: *mut *const c_char) -> c_int;
fn sqlite3_bind_text(stmt: *mut Sqlite3Stmt, idx: c_int, data: *const c_char, n: c_int, destructor: sqlite3_destructor_type) -> c_int;
fn sqlite3_bind_int(stmt: *mut Sqlite3Stmt, idx: c_int, value: c_int) -> c_int;
fn sqlite3_step(stmt: *mut Sqlite3Stmt) -> c_int;
fn sqlite3_changes(db: *mut Sqlite3) -> c_int;
fn sqlite3_column_text(stmt: *mut Sqlite3Stmt, idx: c_int) -> *mut c_char;
fn sqlite3_column_int(stmt: *mut Sqlite3Stmt, idx: c_int) -> c_int;
fn sqlite3_finalize(stmt: *mut Sqlite3Stmt) -> c_int;
fn sqlite3_close(db: *mut Sqlite3) -> c_int;
}

use serde_json;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize)]
struct TodoRequest {
	name: Option<String>,
	task: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TodoTask {
	task: String,
	#[serde(default)]
	completed: bool,
}

pub struct TodoLibrary {
	db: *mut Sqlite3,
}

impl TodoLibrary {
	pub fn new(path: &str) -> Self {
		let mut db: *mut Sqlite3 = ptr::null_mut();
		let c_path = CString::new(path).unwrap();
		unsafe {
			sqlite3_open(c_path.as_ptr(), &mut db);
			let init_sql = "CREATE TABLE IF NOT EXISTS tasks (list_name TEXT, task TEXT, completed INTEGER DEFAULT 0);";
			let c_sql = CString::new(init_sql).unwrap();
			sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut());
		}
		TodoLibrary { db }
	}

	pub fn dispatch(&self, function_name: &str, arguments: &str) -> Result<String, String> {
		let args: TodoRequest = serde_json::from_str(arguments).unwrap_or(TodoRequest { name: None, task: None });
		match function_name {
			"add_todo_task" => {
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.add_todo_task(&name, &task)
			},
			"complete_todo_task" => {
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.set_todo_task_complete(&name, &task, true)
			},
			"delete_todo_task" => {
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				let task = args.task.ok_or(format!("Missing 'task' for {}", function_name))?;
				self.delete_todo_task(&name, &task)
			},
			"get_todo_lists" => {
				self.get_todo_lists()
			},
			"get_todo_tasks" => {
				let name = args.name.ok_or(format!("Missing 'name' for {}", function_name))?;
				self.get_todo_tasks(&name)
			},
			_ => Err(format!("Unknown function: {}", function_name))
		}
	}

	pub fn add_todo_task(&self, name: &str, task: &str) -> Result<String, String> {
		let sql = "INSERT INTO tasks (list_name, task, completed) VALUES (?, ?, 0);";
		let c_sql = CString::new(sql).unwrap();
		let mut stmt: *mut Sqlite3Stmt = ptr::null_mut();

		unsafe {
			if sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) != 0 {
				return Err("failed to prepare statement".into());
			}
			let c_name = CString::new(name).unwrap();
			sqlite3_bind_text(stmt, 1, c_name.as_ptr(), -1, SQLITE_TRANSIENT());

			let c_task = CString::new(task).unwrap();
			sqlite3_bind_text(stmt, 2, c_task.as_ptr(), -1, SQLITE_TRANSIENT());

			let result = sqlite3_step(stmt);

			sqlite3_finalize(stmt);

			if result == SQLITE_DONE {
				Ok(format!("Task '{}' added to list '{}'", task, name))
			} else {
				Err("Could not create task".into())
			}
		}
	}

	pub fn get_todo_lists(&self) -> Result<String, String> {
		let sql = "SELECT DISTINCT list_name FROM tasks;";
		let c_sql = CString::new(sql).unwrap();
		let mut stmt: *mut Sqlite3Stmt = ptr::null_mut();
		let mut lists = Vec::new();

		unsafe {
			sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
			while sqlite3_step(stmt) == SQLITE_ROW {
				let ptr = sqlite3_column_text(stmt, 0);
				if !ptr.is_null() {
					let name = CStr::from_ptr(ptr).to_string_lossy().into_owned();
					lists.push(name);
				}
			}
			sqlite3_finalize(stmt);
		}

		serde_json::to_string(&lists).map_err(|e| e.to_string())
	}

	pub fn get_todo_tasks(&self, name: &str) -> Result<String, String> {
		let sql = "SELECT task, completed FROM tasks WHERE list_name = ?;";
		let c_sql = CString::new(sql).unwrap();
		let mut stmt: *mut Sqlite3Stmt = ptr::null_mut();
		let mut tasks = Vec::new();
		let c_name = CString::new(name).unwrap();

		unsafe {
			sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
			sqlite3_bind_text(stmt, 1, c_name.as_ptr(), -1, SQLITE_TRANSIENT());
			while sqlite3_step(stmt) == SQLITE_ROW {
				let ptr = sqlite3_column_text(stmt, 0);
				let completed: bool = sqlite3_column_int(stmt, 1) == 1;
				if !ptr.is_null() {
					let task = TodoTask{ task: CStr::from_ptr(ptr).to_string_lossy().into_owned(), completed: completed };
					tasks.push(task);
				}
			}
			sqlite3_finalize(stmt);
		}

		serde_json::to_string(&tasks).map_err(|e| e.to_string())
	}

	pub fn set_todo_task_complete(&self, name: &str, task: &str, complete: bool) -> Result<String, String> {
		let sql = "UPDATE tasks SET completed = ? WHERE list_name = ? AND task = ?;";
		let c_sql = CString::new(sql).unwrap();
		let mut stmt: *mut Sqlite3Stmt = ptr::null_mut();
		let c_name = CString::new(name).unwrap();
		let c_task = CString::new(task).unwrap();
		let c_complete: c_int = if complete { 1 } else { 0 };

		unsafe {
			sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
			sqlite3_bind_int(stmt, 1, c_complete);
			sqlite3_bind_text(stmt, 2, c_name.as_ptr(), -1, SQLITE_TRANSIENT());
			sqlite3_bind_text(stmt, 3, c_task.as_ptr(), -1, SQLITE_TRANSIENT());
			let status = sqlite3_step(stmt);
			let changes = sqlite3_changes(self.db);
			sqlite3_finalize(stmt);
			if changes == 0 {
				return Err(format!("Task list not updated, perhaps the task does not exist? (status {})", status));
			}
			if changes > 1 {
				return Err("More than one task was updated, this is not normally a good thing".into());
			}
		}

		Ok("Success".into())
	}

	pub fn delete_todo_task(&self, name: &str, task: &str) -> Result<String, String> {
		let sql = "DELETE FROM tasks WHERE list_name = ? AND task = ? AND completed = 1;";
		let c_sql = CString::new(sql).unwrap();
		let mut stmt: *mut Sqlite3Stmt = ptr::null_mut();
		let c_name = CString::new(name).unwrap();
		let c_task = CString::new(task).unwrap();

		unsafe {
			sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
			sqlite3_bind_text(stmt, 1, c_name.as_ptr(), -1, SQLITE_TRANSIENT());
			sqlite3_bind_text(stmt, 2, c_task.as_ptr(), -1, SQLITE_TRANSIENT());
			let status = sqlite3_step(stmt);
			let changes = sqlite3_changes(self.db);
			sqlite3_finalize(stmt);
			if changes == 0 {
				return Err("Task list not updated: only completed tasks my be deleted from the todo list".into());
			}
			if changes > 1 {
				return Err("It seems more than one task was deleted, this is not normally a good thing".into());
			}
			if status != SQLITE_DONE {
				return Err(format!("Error from todo list, status {}", status));
			}
		}

		Ok("Success".into())
	}
}

impl Drop for TodoLibrary {
	fn drop(&mut self) {
		unsafe {
			sqlite3_close(self.db);
		}
	}
}

