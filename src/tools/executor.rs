use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::{Child, Command},
	select,
	time::{timeout, Duration},
};
use std::process::Stdio;
use std::time::Instant;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ExecuteArgs {
	pub command: String,
}

#[derive(Serialize)]
pub struct ExecuteResults {
	pub output: String,
	pub timed_out: bool,
	pub exit_code: i32,
}

pub struct Executor {
}

#[derive(Debug)]
enum StreamKind {
	Stdout,
	Stderr,
}

#[derive(Debug)]
struct TimedLine {
	at: Instant,
	kind: StreamKind,
	line: String,
}

#[derive(Debug)]
enum RunResult {
	Completed {
		output: Vec<TimedLine>,
		status: std::process::ExitStatus,
	},
	TimedOut {
		output: Vec<TimedLine>,
	},
}

const MAX_LINES: usize = 128;
const MAX_LINE_LEN: usize = 256;

fn push_bounded(buf: &mut Vec<TimedLine>, kind: StreamKind, mut line: String) {
	if line.len() > MAX_LINE_LEN {
		line.truncate(MAX_LINE_LEN);
	}

	if buf.len() == MAX_LINES {
		buf.remove(0);
	}

	buf.push(TimedLine {
		at: Instant::now(),
		kind: kind,
		line,
	});
}

async fn terminate_child_gracefully(child: &mut tokio::process::Child,) {
	// Ask politely first (SIGTERM on Unix)
	let _ = child.start_kill();
	match timeout(Duration::from_secs(1), child.wait()).await {
		Ok(Ok(_status)) => {
			// Child exited cleanly within timeout
		}
		Ok(Err(_e)) => {
			let _ = child.kill().await;
		}
		Err(_) => {
			let _ = child.kill().await;
			let _ = child.wait().await;
		}
	}
}

async fn run_and_capture_with_timeout(mut child: Child, timeout_duration: Duration,) -> std::io::Result<RunResult> {
	let stdout = child.stdout.take().expect("stdout not piped");
	let stderr = child.stderr.take().expect("stderr not piped");

	let mut stdout_reader = BufReader::new(stdout).lines();
	let mut stderr_reader = BufReader::new(stderr).lines();

	let mut output_buf = Vec::new();

	let mut stdout_done = false;
	let mut stderr_done = false;

	let read_fut = async {
		while !stdout_done || !stderr_done {
			select! {
				line = stdout_reader.next_line(), if !stdout_done => {
					match line? {
						Some(l) => push_bounded(&mut output_buf, StreamKind::Stdout, l),
						None => stdout_done = true,
					}
				}

				line = stderr_reader.next_line(), if !stderr_done => {
					match line? {
						Some(l) => push_bounded(&mut output_buf, StreamKind::Stderr, l),
						None => stderr_done = true,
					}
				}
			}
		}
		Ok::<(), std::io::Error>(())
	};

	match timeout(timeout_duration, read_fut).await {
		Ok(Ok(())) => {
			let status = child.wait().await?;
			Ok(RunResult::Completed {
				output: output_buf,
				status,
			})
		}

		Ok(Err(e)) => Err(e),

		Err(_) => {
			terminate_child_gracefully(&mut child).await;
			Ok(RunResult::TimedOut {
				output: output_buf,
			})
		}
	}
}

fn lines_with_offsets(started_at: Instant, lines: &[TimedLine]) -> String {
	lines
		.iter()
		.map(|tl| {
			let millis = match tl.at.checked_duration_since(started_at) {
				Some(d) => d.as_millis() as i128,
				None => -(started_at.duration_since(tl.at).as_millis() as i128),
			};

			format!("{:>5}| {}", millis, tl.line)
		})
		.collect::<Vec<_>>()
		.join("\n")
}

impl Executor {
	pub async fn execute(args: ExecuteArgs) -> Result<ExecuteResults, String> {
		let started_at: Instant = Instant::now();
		let child = Command::new("sh")
			.arg("-c")
			.arg(args.command)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.expect("Failed to spawn command");
		let child_result = run_and_capture_with_timeout(child, Duration::from_secs(120)).await;
		match child_result {
			Ok(run_result) => {
				match run_result {
					RunResult::TimedOut{ output } => {
						Ok(ExecuteResults{ output: lines_with_offsets(started_at, &output), exit_code: 137, timed_out: true })
					},
					RunResult::Completed{ output, status } => {
						let exit_code = if status.success() { status.code().unwrap_or(-1) } else { -1 };
						Ok(ExecuteResults{ output: lines_with_offsets(started_at, &output), exit_code: exit_code, timed_out: false })
					},
				}
			},
			Err(err) => {
				Err(err.to_string())
			},
		}
	}
}


