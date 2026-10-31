//! Private D-Bus transport fixture. Never talks to the desktop's real portal.
use super::*;
use std::sync::atomic::AtomicU32;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

struct DiagnosticSession(flume::Sender<()>);
#[zbus::interface(name = "org.freedesktop.portal.Session")]
impl DiagnosticSession {
    fn close(&self) { let _ = self.0.try_send(()); }
}

struct DiagnosticPortal {
    closed: flume::Sender<()>,
    peers: flume::Sender<std::os::unix::net::UnixStream>,
}

async fn diagnostic_response(connection: &zbus::Connection, header: zbus::message::Header<'_>,
    options: &HashMap<String, OwnedValue>, result: HashMap<&str, Value<'_>>) -> zbus::fdo::Result<OwnedObjectPath> {
    let sender = header.sender().unwrap();
    let id = sender.trim_start_matches(':').replace('.', "_");
    let token = options["handle_token"].downcast_ref::<&str>().unwrap();
    let path: OwnedObjectPath = format!("/org/freedesktop/portal/desktop/request/{id}/{token}").try_into().unwrap();
    connection.emit_signal(Some(sender.as_str()), path.clone(), "org.freedesktop.portal.Request", "Response",
        &(0u32, result)).await.unwrap();
    Ok(path)
}

#[zbus::interface(name = "org.freedesktop.portal.ScreenCast")]
impl DiagnosticPortal {
    #[zbus(property)]
    fn version(&self) -> u32 { 5 }

    async fn create_session(&self, options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>) -> zbus::fdo::Result<OwnedObjectPath> {
        let id = header.sender().unwrap().trim_start_matches(':').replace('.', "_");
        let token = options["session_handle_token"].downcast_ref::<&str>().unwrap();
        let session: OwnedObjectPath = format!("/org/freedesktop/portal/desktop/session/{id}/{token}").try_into().unwrap();
        connection.object_server().at(session.clone(), DiagnosticSession(self.closed.clone())).await.unwrap();
        diagnostic_response(connection, header, &options, HashMap::from([("session_handle", Value::from(session))])).await
    }

    async fn select_sources(&self, _session: OwnedObjectPath, options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>) -> zbus::fdo::Result<OwnedObjectPath> {
        diagnostic_response(connection, header, &options, HashMap::new()).await
    }

    async fn start(&self, _session: OwnedObjectPath, _parent: &str, options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>) -> zbus::fdo::Result<OwnedObjectPath> {
        let streams = vec![(71u32, HashMap::from([("source_type", Value::from(1u32)), ("mapping_id", Value::from("SYNTHETIC-A"))])),
            (72u32, HashMap::from([("source_type", Value::from(1u32)), ("mapping_id", Value::from("SYNTHETIC-B"))]))];
        diagnostic_response(connection, header, &options, HashMap::from([
            ("streams", Value::from(streams)), ("restore_token", Value::from("PRIVATE-CANARY-NEVER-LOG"))])).await
    }

    fn open_pipe_wire_remote(&self, _session: OwnedObjectPath, _options: HashMap<String, OwnedValue>) -> zbus::zvariant::OwnedFd {
        let (fd, peer) = std::os::unix::net::UnixStream::pair().unwrap();
        self.peers.send(peer).unwrap();
        zbus::zvariant::OwnedFd::from(OwnedFd::from(fd))
    }
}

#[test]
fn diagnostic_authorization_runs_on_private_bus() {
    let data = std::env::temp_dir().join(format!("apparent-portal-diag-{}", crate::make_bundle_id()));
    std::fs::create_dir(&data).unwrap();
    let output = std::process::Command::new("dbus-run-session").arg("--").arg(std::env::current_exe().unwrap())
        .args(["--exact", "linux::portal::protocol_tests::diagnostic_authorization_private_bus_fixture", "--ignored", "--nocapture"])
        .env("APPARENT_TEST_OWNED_REQUEST_BUS", "1").env("APPARENT_PORTAL_DIAGNOSTICS", "1")
        .env("XDG_DATA_HOME", &data).output().unwrap();
    std::fs::remove_dir_all(data).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(!stderr.contains("PRIVATE-CANARY-NEVER-LOG"));
    let records: Vec<serde_json::Value> = stderr.lines().filter_map(|line| line.split_once("portal-resource "))
        .map(|(_, json)| serde_json::from_str(json).unwrap()).collect();
    let streams: Vec<_> = records.iter().filter(|r| r["event"] == "held_fd" && r["role"] == "stream").collect();
    assert_eq!(streams.len(), 4);
    let generations: std::collections::HashSet<_> = streams.iter().map(|r| r["generation"].as_u64().unwrap()).collect();
    assert_eq!(generations.len(), 2);
    for held in streams {
        assert_eq!(held["identity_status"], "OBSERVED");
        assert!(held["session"].as_str().unwrap().starts_with("/org/freedesktop/portal/desktop/session/"));
        assert!(held["mapping_sha256"].as_str().unwrap().len() == 64);
        assert!(records.iter().any(|r| r["event"] == "media_binding" && r["generation"] == held["generation"] && r["fd"] == held["fd"]));
        assert!(records.iter().any(|r| r["event"] == "producer_fd_dropped" && r["generation"] == held["generation"] && r["fd"] == held["fd"]));
        assert!(records.iter().any(|r| r["event"] == "held_fd" && r["role"] == "remote" && r["generation"] == held["generation"]
            && r["fd"] == held["parent_fd"] && r["socket_cookie"] == held["socket_cookie"] && r["target"] == held["target"]));
    }
    if let Ok(path) = std::env::var("APPARENT_DIAGNOSTIC_TEST_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "only in the subprocess's isolated bus"]
fn diagnostic_authorization_private_bus_fixture() {
    assert_eq!(std::env::var("APPARENT_TEST_OWNED_REQUEST_BUS").as_deref(), Ok("1"));
    futures_lite::future::block_on(async {
        let (closed, closes) = flume::bounded(4);
        let (peers, remote_peers) = flume::bounded(4);
        let _service = zbus::connection::Builder::session().unwrap().name("org.freedesktop.portal.Desktop").unwrap()
            .serve_at("/org/freedesktop/portal/desktop", DiagnosticPortal { closed, peers }).unwrap().build().await.unwrap();
        for _ in 0..2 {
            let worker = std::thread::spawn(|| {
                let (streams, token) = portal_screencasts_until(false, true, "synthetic-test-only", &AtomicBool::new(false),
                    std::time::Instant::now() + Duration::from_secs(3)).unwrap();
                assert_eq!(token.as_deref(), Some("PRIVATE-CANARY-NEVER-LOG"));
                assert_eq!(streams.len(), 2);
                for stream in &streams { stream.diagnostic_media_binding(); }
                drop(streams);
            });
            let mut peer = remote_peers.recv_async().await.unwrap();
            worker.join().unwrap();
            peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            use std::io::Read;
            assert_eq!(peer.read(&mut [0]).unwrap(), 0, "actual last descriptor release must produce EOF");
            assert!(closes.recv_timeout(Duration::from_secs(2)).is_ok());
        }
    });
}

struct FakeRequest(Arc<AtomicBool>);
#[zbus::interface(name = "org.freedesktop.portal.Request")]
impl FakeRequest {
    fn close(&self) { self.0.store(true, Ordering::Release); }
}

struct FakePortal {
    status: Arc<AtomicU32>,
    hold: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    called: flume::Sender<()>,
}
#[zbus::interface(name = "org.freedesktop.portal.ScreenCast")]
impl FakePortal {
    async fn select_sources(&self, session: OwnedObjectPath, options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>) -> zbus::fdo::Result<OwnedObjectPath> {
        assert_eq!(session.as_str(), "/synthetic/session");
        assert_eq!(options["types"].downcast_ref::<u32>().unwrap(), 1);
        assert!(!options["multiple"].downcast_ref::<bool>().unwrap());
        let token = options["handle_token"].downcast_ref::<&str>().unwrap();
        let sender = header.sender().unwrap();
        let id = sender.trim_start_matches(':').replace('.', "_");
        let path: OwnedObjectPath = format!("/org/freedesktop/portal/desktop/request/{id}/{token}").try_into().unwrap();
        connection.object_server().at(path.clone(), FakeRequest(Arc::clone(&self.closed))).await.unwrap();
        self.called.try_send(()).unwrap();
        if !self.hold.load(Ordering::Acquire) {
            connection.emit_signal(Some(sender.as_str()), path.clone(), "org.freedesktop.portal.Request", "Response",
                &(self.status.load(Ordering::Acquire), HashMap::<String, OwnedValue>::new())).await.unwrap();
        }
        Ok(path)
    }
}

#[test]
fn owned_request_runs_on_private_bus() {
    let output = std::process::Command::new("dbus-run-session").arg("--").arg(std::env::current_exe().unwrap())
        .args(["--exact", "linux::portal::protocol_tests::owned_request_private_bus_fixture", "--ignored", "--nocapture"])
        .env("APPARENT_TEST_OWNED_REQUEST_BUS", "1").env("APPARENT_PORTAL_DIAGNOSTICS", "1")
        .output().expect("private dbus-run-session fixture");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let records: Vec<serde_json::Value> = String::from_utf8_lossy(&output.stderr).lines()
        .filter_map(|line| line.split_once("portal-resource "))
        .map(|(_, json)| serde_json::from_str(json).unwrap()).collect();
    assert!(records.iter().any(|r| r["event"] == "request_pending"),
        "actual private-bus request must export its owned identity, not just close text");
    for record in &records {
        assert_eq!(record["schema"], 1);
        assert!(record["bus"].as_str().unwrap().starts_with(':'));
        assert!(record["generation"].as_u64().unwrap() > 0);
        assert!(!record.to_string().contains("restore_token"));
    }
}

#[test]
#[ignore = "invoked only by owned_request_runs_on_private_bus in its private bus"]
fn owned_request_private_bus_fixture() {
    assert_eq!(std::env::var("APPARENT_TEST_OWNED_REQUEST_BUS").as_deref(), Ok("1"), "refuse real desktop bus");
    futures_lite::future::block_on(async {
        let status = Arc::new(AtomicU32::new(0));
        let hold = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));
        let (called, calls) = flume::bounded(1);
        let _service = zbus::connection::Builder::session().unwrap().name("org.freedesktop.portal.Desktop").unwrap()
            .serve_at("/org/freedesktop/portal/desktop", FakePortal { status: status.clone(), hold: hold.clone(), closed: closed.clone(), called }).unwrap()
            .build().await.unwrap();
        let client = zbus::Connection::session().await.unwrap();
        let proxy = zbus::Proxy::new(&client, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.ScreenCast").await.unwrap();
        for response in [0, 1, 2] {
            status.store(response, Ordering::Release);
            let request = OwnedPortalRequest::new(&client).unwrap();
            let pending = std::cell::RefCell::new(None);
            let options = HashMap::from([("handle_token", Value::from(request.token.as_str())), ("types", Value::from(1u32)), ("multiple", Value::from(false))]);
            let result = request.perform(&proxy, "SelectSources", &(OwnedObjectPath::try_from("/synthetic/session").unwrap(), options), &pending).await;
            calls.recv_async().await.unwrap();
            assert_eq!(result.is_ok(), response == 0);
            if response == 1 { assert!(user_cancelled(&result.unwrap_err())); }
            assert!(pending.borrow().is_none(), "Response ends owned request");
        }
        hold.store(true, Ordering::Release);
        let request = OwnedPortalRequest::new(&client).unwrap();
        let pending = std::cell::RefCell::new(None);
        let options = HashMap::from([("handle_token", Value::from(request.token.as_str())), ("types", Value::from(1u32)), ("multiple", Value::from(false))]);
        let body = (OwnedObjectPath::try_from("/synthetic/session").unwrap(), options);
        let result = futures_lite::future::race(request.perform(&proxy, "SelectSources", &body, &pending), async {
            calls.recv_async().await.unwrap();
            Err(anyhow!("synthetic cancellation with no Response"))
        }).await;
        assert!(result.is_err());
        assert_eq!(pending.borrow().as_ref(), Some(&request.path));
        // Exercise the same owned Request.Close operation used by detached production cleanup.
        cleanup_portal(async {
            client.call_method(Some("org.freedesktop.portal.Desktop"), pending.take().unwrap(),
                Some("org.freedesktop.portal.Request"), "Close", &()).await.unwrap();
        }, async {}, Duration::from_secs(1)).await;
        assert!(closed.load(Ordering::Acquire), "dropping the pending future alone is not Request.Close");
    });
}
