//! Private D-Bus transport fixture. Never talks to the desktop's real portal.
use super::*;
use std::sync::atomic::AtomicU32;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

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
    let status = std::process::Command::new("dbus-run-session").arg("--").arg(std::env::current_exe().unwrap())
        .args(["--exact", "linux::portal::protocol_tests::owned_request_private_bus_fixture", "--ignored", "--nocapture"])
        .env("APPARENT_TEST_OWNED_REQUEST_BUS", "1").status().expect("private dbus-run-session fixture");
    assert!(status.success());
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
