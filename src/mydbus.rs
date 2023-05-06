use dbus::blocking::Connection;
use std::error::Error;
use std::time::Duration;

const BUSNAME: &str = "org.kde.Solid.PowerManagement.PolicyAgent";
const PATH: &str = "/org/kde/Solid/PowerManagement/PolicyAgent";
const INTERFACE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";

pub struct Cookies {
    conn: Connection,
    cook1: Option<u32>,
    cook2: Option<u32>,
    cook3: Option<u32>,
    pub state: bool,
}

impl Cookies {
    pub fn new() -> Result<Cookies, Box<dyn Error>> {
        let conn = Connection::new_session()?;
        Ok(Cookies {
            conn,
            cook1: None,
            cook2: None,
            cook3: None,
            state: false,
        })
    }

    pub fn get_status() -> Result<String, Box<dyn Error>> {
        let conn = Connection::new_session()?;
        let proxy = conn.with_proxy(BUSNAME, PATH, Duration::from_secs(5));
        let (names,): (Vec<(String, String)>,) =
            proxy.method_call(INTERFACE, "ListInhibitions", ())?;

        let mut result = String::new();
        if names.len() > 0 {
            let mut last = String::new();
            for name in names {
                if name.0 == last {
                    continue;
                }
                result.push_str(&name.0);
                result.push_str(": ");
                result.push_str(&name.1);
                result.push_str(".\n");
                last = name.0;
            }
            result.pop();
        }
        Ok(result)
    }
    pub fn add_inhibit(&mut self, name: &str, desc: &str) {
        let proxy = self.conn.with_proxy(BUSNAME, PATH, Duration::from_secs(5));
        // InterruptSession = 1,
        // ChangeProfile = 2,
        // ChangeScreenSettings = 4
        let (cook1,): (u32,) = proxy
            .method_call(INTERFACE, "AddInhibition", (1 as u32, name, desc))
            .unwrap();
        let (cook2,): (u32,) = proxy
            .method_call(INTERFACE, "AddInhibition", (2 as u32, name, desc))
            .unwrap();
        let (cook3,): (u32,) = proxy
            .method_call(INTERFACE, "AddInhibition", (4 as u32, name, desc))
            .unwrap();
        self.cook1 = Some(cook1);
        self.cook2 = Some(cook2);
        self.cook3 = Some(cook3);
    }

    pub fn rel_inhibit(&mut self) {
        let proxy = self.conn.with_proxy(BUSNAME, PATH, Duration::from_secs(5));
        if let Some(v) = self.cook1 {
            proxy
                .method_call::<(), _, _, _>(INTERFACE, "ReleaseInhibition", (v,))
                .unwrap();
            self.cook1 = None;
        }
        if let Some(v) = self.cook2 {
            proxy
                .method_call::<(), _, _, _>(INTERFACE, "ReleaseInhibition", (v,))
                .unwrap();
            self.cook2 = None;
        }
        if let Some(v) = self.cook3 {
            proxy
                .method_call::<(), _, _, _>(INTERFACE, "ReleaseInhibition", (v,))
                .unwrap();
            self.cook3 = None;
        }
    }
}
