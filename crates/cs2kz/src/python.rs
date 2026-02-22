use std::bstr::ByteStr;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use std::{fmt, io, mem};

use futures_util::FutureExt as _;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::OnceCell;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;
use tokio_util::time::FutureExt as _;
use tracing::Instrument as _;

#[derive(Debug)]
pub struct Python<Request, Response> {
    script_path: PathBuf,
    process: Child,
    process_stdout: BufReader<ChildStdout>,
    process_stderr_reader_task: AbortOnDropHandle<io::Result<()>>,
    _request: PhantomData<fn() -> Request>,
    _response: PhantomData<fn() -> Response>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum PythonResponse<T> {
    Success(T),
    Error {
        #[serde(rename = "error")]
        message: String,
    },
}

impl<Request, Response> Python<Request, Response> {
    pub async fn new(script_path: PathBuf) -> io::Result<Self> {
        let (mut process, process_stderr_reader_task) = spawn_script(&script_path).await?;
        let process_stdout = process
            .stdout
            .take()
            .map(BufReader::new)
            .expect("we only take stdout once");

        Ok(Self {
            script_path,
            process,
            process_stdout,
            process_stderr_reader_task: AbortOnDropHandle::new(process_stderr_reader_task),
            _request: PhantomData,
            _response: PhantomData,
        })
    }

    #[tracing::instrument(skip(self), err)]
    pub async fn send_request(&mut self, request: &Request) -> io::Result<Response>
    where
        Request: fmt::Debug + serde::Serialize,
        Response: for<'de> serde::Deserialize<'de>,
    {
        let mut serialized_request =
            serde_json::to_vec(request).expect("requests should serialize to JSON");
        serialized_request.push(b'\n');

        let mut serialized_response = Vec::with_capacity(128);

        'outer: loop {
            serialized_response.clear();

            if let Some(exit_status) = self.process.try_wait()? {
                tracing::warn!(?exit_status, "python process exited");
                self.reset().await?;
                continue;
            }

            tracing::trace!(
                request = str::from_utf8(&serialized_request).unwrap(),
                "writing request to python stdin"
            );
            {
                let stdin = self.process.stdin.as_mut().expect("we never close stdin");
                stdin.write_all(&serialized_request[..]).await?;
                stdin.flush().await?;
            }

            tracing::trace!("reading response from python stdout");
            for _ in 0..3 {
                match self
                    .process_stdout
                    .read_until(b'\n', &mut serialized_response)
                    .timeout(Duration::from_secs(10))
                    .await
                {
                    Ok(Ok(_)) => break,
                    Ok(Err(err)) => {
                        tracing::error!(%err, "failed to read from stdout");
                        self.reset().await?;
                        continue 'outer;
                    },
                    Err(_elapsed) => {
                        tracing::warn!(
                            stdout = ?ByteStr::new(self.process_stdout.buffer()),
                            response_buf = ?ByteStr::new(&serialized_response),
                            "still waiting for response",
                        );
                    },
                }
            }

            break if serialized_response.is_empty() {
                Err(io::Error::new(io::ErrorKind::TimedOut, "did not complete request in time"))
            } else {
                serde_json::from_slice::<PythonResponse<Response>>(&serialized_response[..])
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
                    .and_then(|response| match response {
                        PythonResponse::Success(res) => Ok(res),
                        PythonResponse::Error { message } => Err(io::Error::other(message)),
                    })
            };
        }
    }

    pub async fn reset(&mut self) -> io::Result<()> {
        let (process, process_stderr_reader_task) = spawn_script(&self.script_path).await?;
        self.process = process;
        self.process_stdout = self
            .process
            .stdout
            .take()
            .map(BufReader::new)
            .expect("we only take stdout once");

        let old_process_stderr_reader_task = mem::replace(
            &mut self.process_stderr_reader_task,
            AbortOnDropHandle::new(process_stderr_reader_task),
        );

        if let Some(Ok(Err(err))) = old_process_stderr_reader_task.now_or_never() {
            tracing::error!(%err, "python stderr task encountered an error");
        }

        Ok(())
    }
}

async fn spawn_script(path: &Path) -> io::Result<(Child, task::JoinHandle<io::Result<()>>)> {
    let span = tracing::debug_span!("python", script_path = %path.display());
    let executable_name = resolve_executable_name().await?;
    let mut child = Command::new(executable_name)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("we only take stderr once");
    let task = task::spawn(read_from_stderr(stderr).instrument(span));

    Ok((child, task))
}

async fn resolve_executable_name() -> io::Result<&'static OsStr> {
    #[cfg(unix)]
    const EXECUTABLE_NAMES: &[&str] = &["python3", "python", "py"];

    #[cfg(windows)]
    const EXECUTABLE_NAMES: &[&str] = &["python3.exe", "python.exe", "py.exe"];

    static EXECUTABLE_NAME: OnceCell<&OsStr> = OnceCell::const_new();

    EXECUTABLE_NAME
        .get_or_try_init(async || {
            for name in EXECUTABLE_NAMES {
                if Command::new(name)
                    .arg("--version")
                    .output()
                    .await?
                    .status
                    .success()
                {
                    return Ok(OsStr::new(name));
                }
            }

            Err(io::Error::other("failed to find suitable python executable"))
        })
        .await
        .copied()
}

async fn read_from_stderr(stderr: ChildStderr) -> io::Result<()> {
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();

    while let 1.. = stderr.read_line(&mut line).await? {
        if let Some(c) = line.pop()
            && c != '\n'
        {
            line.push(c);
        }

        tracing::debug!(line);
        line.clear();
    }

    Ok(())
}
