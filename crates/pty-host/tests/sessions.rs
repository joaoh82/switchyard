//! End-to-end tests against real processes in real PTYs. These run on Linux, macOS and Windows
//! in CI, which is the point: the PTY layer is where the platforms differ most.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pty_host::{HostError, HostEvent, LaunchPlan, PtyHost, SessionId, SessionState, TermSize};

const TIMEOUT: Duration = Duration::from_secs(20);

fn host() -> (PtyHost, Receiver<HostEvent>) {
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let host = PtyHost::new(Arc::new(move |event| {
        let _ = tx.lock().unwrap().send(event);
    }));
    (host, rx)
}

/// Run `script` with the platform's shell.
fn shell(script: &str) -> LaunchPlan {
    let (program, flag) = if cfg!(windows) {
        ("cmd.exe", "/C")
    } else {
        ("/bin/sh", "-c")
    };
    LaunchPlan {
        program: program.into(),
        args: vec![flag.into(), script.into()],
        cwd: None,
        env: vec![],
        clear_env: false,
        size: TermSize { cols: 80, rows: 24 },
    }
}

/// A process that stays alive until killed.
fn long_running() -> LaunchPlan {
    shell(if cfg!(windows) {
        "ping -n 60 127.0.0.1"
    } else {
        "sleep 60"
    })
}

/// Collects everything a session sends to one viewer.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Capture {
    fn sink(&self) -> pty_host::OutputSink {
        let buf = Arc::clone(&self.0);
        Box::new(move |bytes| {
            buf.lock().unwrap().extend_from_slice(bytes);
            true
        })
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }

    fn wait_for(&self, needle: &str) {
        let start = Instant::now();
        while !self.text().contains(needle) {
            assert!(
                start.elapsed() < TIMEOUT,
                "timed out waiting for {needle:?}; got {:?}",
                self.text()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn wait_for_exit(events: &Receiver<HostEvent>, id: &SessionId) -> pty_host::ExitInfo {
    loop {
        match events
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for exit")
        {
            HostEvent::Exited { id: exited, exit } if &exited == id => return exit,
            HostEvent::Exited { .. } => {}
        }
    }
}

#[test]
fn streams_output_then_reports_the_exit_code() {
    let (host, events) = host();
    let session = host.spawn(shell("echo hello-from-pty&& exit 3")).unwrap();
    let capture = Capture::default();
    host.attach(&session.id, capture.sink()).unwrap();

    let exit = wait_for_exit(&events, &session.id);

    // All output is delivered before `Exited` is announced.
    assert!(
        capture.text().contains("hello-from-pty"),
        "{:?}",
        capture.text()
    );
    assert_eq!(exit.code, 3);
    assert!(!exit.success);
    assert!(matches!(
        host.info(&session.id).unwrap().state,
        SessionState::Exited { .. }
    ));
}

#[test]
fn a_late_viewer_gets_a_snapshot_of_what_it_missed() {
    let (host, events) = host();
    let session = host
        .spawn(shell("echo printed-before-anyone-watched"))
        .unwrap();
    wait_for_exit(&events, &session.id);

    let capture = Capture::default();
    host.attach(&session.id, capture.sink()).unwrap();

    // Delivered synchronously by `attach`, even though the process is long gone.
    assert!(
        capture.text().contains("printed-before-anyone-watched"),
        "{:?}",
        capture.text()
    );
}

#[test]
fn detached_viewers_stop_receiving() {
    let (host, events) = host();
    let script = if cfg!(windows) {
        "ping -n 2 127.0.0.1 >NUL&& echo second-part"
    } else {
        "sleep 1; echo second-part"
    };
    let session = host.spawn(shell(script)).unwrap();
    let (stays, leaves) = (Capture::default(), Capture::default());
    host.attach(&session.id, stays.sink()).unwrap();
    let leaving = host.attach(&session.id, leaves.sink()).unwrap();
    host.detach(&session.id, leaving).unwrap();

    wait_for_exit(&events, &session.id);

    assert!(stays.text().contains("second-part"));
    assert!(!leaves.text().contains("second-part"));
}

#[cfg(unix)]
#[test]
fn input_reaches_the_process() {
    let (host, events) = host();
    let session = host.spawn(shell("read line; echo \"got:$line\"")).unwrap();
    let capture = Capture::default();
    host.attach(&session.id, capture.sink()).unwrap();

    host.write(&session.id, b"ping\n").unwrap();

    capture.wait_for("got:ping");
    assert!(wait_for_exit(&events, &session.id).success);
}

#[cfg(unix)]
#[test]
fn the_process_sees_the_terminal_size_and_resizes() {
    let (host, events) = host();
    let mut plan = shell("stty size; read _; stty size");
    plan.size = TermSize {
        cols: 100,
        rows: 30,
    };
    let session = host.spawn(plan).unwrap();
    let capture = Capture::default();
    host.attach(&session.id, capture.sink()).unwrap();
    capture.wait_for("30 100");

    host.resize(
        &session.id,
        TermSize {
            cols: 132,
            rows: 43,
        },
    )
    .unwrap();
    host.write(&session.id, b"\n").unwrap();

    capture.wait_for("43 132");
    assert_eq!(
        host.info(&session.id).unwrap().size,
        TermSize {
            cols: 132,
            rows: 43
        }
    );
    wait_for_exit(&events, &session.id);
}

#[test]
fn kill_ends_a_running_session() {
    let (host, events) = host();
    let session = host.spawn(long_running()).unwrap();
    assert_eq!(host.info(&session.id).unwrap().state, SessionState::Running);

    host.kill(&session.id).unwrap();

    assert!(!wait_for_exit(&events, &session.id).success);
    assert!(matches!(
        host.kill(&session.id),
        Err(HostError::SessionExited(_))
    ));
    assert!(matches!(
        host.write(&session.id, b"x"),
        Err(HostError::SessionExited(_))
    ));
}

#[test]
fn sessions_are_listed_until_removed() {
    let (host, _events) = host();
    let a = host.spawn(long_running()).unwrap();
    let b = host.spawn(long_running()).unwrap();
    let mut expected = vec![a.id.clone(), b.id.clone()];
    expected.sort();
    assert_eq!(
        host.list().into_iter().map(|s| s.id).collect::<Vec<_>>(),
        expected
    );

    host.remove(&a.id).unwrap();

    assert_eq!(
        host.list().into_iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![b.id]
    );
    assert!(matches!(
        host.info(&a.id),
        Err(HostError::UnknownSession(_))
    ));
}

#[test]
fn a_missing_program_is_a_spawn_error() {
    let (host, _events) = host();
    let mut plan = shell("");
    plan.program = "switchyard-no-such-program".into();
    match host.spawn(plan) {
        Err(HostError::Spawn { program, .. }) => assert_eq!(program, "switchyard-no-such-program"),
        other => panic!("expected a spawn error, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn the_plan_controls_the_environment() {
    let (host, events) = host();
    let mut plan = shell("echo \"[$SY_TEST|$TERM|${HOME:-unset}]\"");
    plan.clear_env = true;
    plan.env = vec![("SY_TEST".into(), "yes".into())];
    let session = host.spawn(plan).unwrap();
    let capture = Capture::default();
    host.attach(&session.id, capture.sink()).unwrap();
    wait_for_exit(&events, &session.id);
    assert!(
        capture.text().contains("[yes|xterm-256color|unset]"),
        "{:?}",
        capture.text()
    );
}
