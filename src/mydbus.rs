use dbus::blocking::Connection;
use std::error::Error;
use std::time::Duration;

const BUSNAME: &str = "org.kde.Solid.PowerManagement.PolicyAgent";
const PATH: &str = "/org/kde/Solid/PowerManagement/PolicyAgent";
const INTERFACE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";

pub struct PMCookies {
    connection: Connection,
    cookies: [Option<u32>; 3],
    inhibited: bool,
}

impl PMCookies {
    pub fn new() -> Result<PMCookies, Box<dyn Error>> {
        let connection = Connection::new_session()?;
        Ok(PMCookies {
            connection,
            cookies: [None; 3],
            inhibited: false,
        })
    }

    pub fn get_inhibitions() -> Result<Vec<String>, Box<dyn Error>> {
        let connection = Connection::new_session()?;
        let proxy = connection.with_proxy(BUSNAME, PATH, Duration::from_secs(5));
        let (apps,): (Vec<(String, String)>,) =
            proxy.method_call(INTERFACE, "ListInhibitions", ())?;

        let mut result = Vec::new();
        for (name, reason) in apps {
            result.push(format!("{}: {}", name, reason));
        }
        result.sort_unstable();
        result.dedup();

        Ok(result)
    }

    pub fn get_inhibit_status(&self) -> bool {
        self.inhibited
    }

    pub fn add_inhibit(&mut self, name: &str, reason: &str) {
        let proxy = self
            .connection
            .with_proxy(BUSNAME, PATH, Duration::from_secs(5));

        // https://github.com/KDE/solid-power/blob/master/src/inhibitions_p.h
        // InterruptSession = 1,
        // ChangeProfile = 2,
        // ChangeScreenSettings = 4
        let required_policy: [u32; 3] = [1, 2, 4];

        for i in 0..3 {
            let (cookie,): (u32,) = proxy
                .method_call(
                    INTERFACE,
                    "AddInhibition",
                    (required_policy[i], name, reason),
                )
                .unwrap();
            self.cookies[i] = Some(cookie);
        }

        self.inhibited = true;
    }

    pub fn release_inhibit(&mut self) {
        let proxy = self
            .connection
            .with_proxy(BUSNAME, PATH, Duration::from_secs(5));

        for cookie in &mut self.cookies {
            if let Some(v) = cookie.take() {
                proxy
                    .method_call::<(), _, _, _>(INTERFACE, "ReleaseInhibition", (v,))
                    .unwrap();
            }
        }

        self.inhibited = false;
    }
}
