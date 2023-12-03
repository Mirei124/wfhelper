use std::env::var;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};
use wayland_protocols_plasma::idle::client::{org_kde_kwin_idle, org_kde_kwin_idle_timeout};
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_manager_v1;
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1::{
    self, Mode,
};

pub fn do_off(wait_time: Option<u32>) {
    // millisec
    let wait_time = wait_time.unwrap_or(500);
    let session_type = var("XDG_SESSION_TYPE").unwrap();
    match &session_type[..] {
        "wayland" => wayland_off(wait_time),
        "x11" => {
            sleep(Duration::from_millis(wait_time as u64));
            Command::new("xset")
                .args(["dpms", "force", "standby"])
                .status()
                .unwrap();
        }
        _ => {
            panic!("No match for XDG_SESSION_TYPE: {session_type}");
        }
    }
}

fn wayland_off(wait_time: u32) {
    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = State {
        manager: None,
        seat: None,
        outputs: Vec::new(),
        idle: None,
        idle_new: None,
        resumed: false,
    };

    event_queue.blocking_dispatch(&mut state).unwrap();

    if state.manager.is_none() {
        panic!("Current wayland compositor not support zwlr_output_power_manager_v1");
    }

    if state.idle.is_none() && state.idle_new.is_none() {
        panic!(
        "Current wayland compositor supports neither org_kde_kwin_idle nor ext_idle_notifier_v1"
    );
    }

    let mut idle_time = None;
    let mut idle_time_new = None;
    if state.idle.is_some() {
        idle_time = Some(state.idle.as_ref().unwrap().get_idle_timeout(
            state.seat.as_ref().unwrap(),
            wait_time,
            &qhandle,
            (),
        ));
    } else {
        idle_time_new = Some(state.idle_new.as_ref().unwrap().get_idle_notification(
            wait_time,
            state.seat.as_ref().unwrap(),
            &qhandle,
            (),
        ));
    }
    while !state.resumed {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
    if idle_time.is_some() {
        idle_time.unwrap().release();
    }
    if idle_time_new.is_some() {
        idle_time_new.unwrap().destroy();
    }
    event_queue.roundtrip(&mut state).unwrap();
}

struct State {
    manager: Option<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    outputs: Vec<Option<wl_output::WlOutput>>,
    idle: Option<org_kde_kwin_idle::OrgKdeKwinIdle>,
    idle_new: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
    resumed: bool,
}

impl State {
    fn set_output_off(&self, qhandle: &QueueHandle<State>) {
        let manager = self.manager.as_ref().unwrap();
        for output in &self.outputs {
            let output_power = manager.get_output_power(output.as_ref().unwrap(), qhandle, ());
            output_power.set_mode(Mode::Off);
            output_power.destroy();
        }
    }
    fn set_output_on(&self, qhandle: &QueueHandle<State>) {
        let manager = self.manager.as_ref().unwrap();
        for output in &self.outputs {
            let output_power = manager.get_output_power(output.as_ref().unwrap(), qhandle, ());
            output_power.set_mode(Mode::On);
            output_power.destroy();
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "zwlr_output_power_manager_v1" => {
                    let manager = proxy
                        .bind::<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, _, _>(
                            name,
                            version,
                            qhandle,
                            (),
                        );
                    state.manager = Some(manager);
                }
                "wl_seat" => {
                    let seat = proxy.bind::<wl_seat::WlSeat, _, _>(name, version, qhandle, ());
                    state.seat = Some(seat);
                }
                "org_kde_kwin_idle" => {
                    let idle = proxy.bind::<org_kde_kwin_idle::OrgKdeKwinIdle, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.idle = Some(idle);
                }
                "wl_output" => {
                    let output =
                        proxy.bind::<wl_output::WlOutput, _, _>(name, version, qhandle, ());
                    state.outputs.push(Some(output));
                }
                "ext_idle_notifier_v1" => {
                    let idle = proxy.bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.idle_new = Some(idle);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout,
        event: <org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            org_kde_kwin_idle_timeout::Event::Idle => {
                state.set_output_off(qhandle);
            }
            org_kde_kwin_idle_timeout::Event::Resumed => {
                state.resumed = true;
                state.set_output_on(qhandle);
            }
            _ => {
                unreachable!("{event:?}");
            }
        }
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: <ext_idle_notification_v1::ExtIdleNotificationV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                state.set_output_off(qhandle);
            }
            ext_idle_notification_v1::Event::Resumed => {
                state.resumed = true;
                state.set_output_on(qhandle);
            }
            _ => {
                unreachable!("{event:?}");
            }
        }
    }
}

impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _event: <ext_idle_notifier_v1::ExtIdleNotifierV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: <wl_output::WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
        _event: <zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<org_kde_kwin_idle::OrgKdeKwinIdle, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_idle::OrgKdeKwinIdle,
        _event: <org_kde_kwin_idle::OrgKdeKwinIdle as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<zwlr_output_power_v1::ZwlrOutputPowerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_power_v1::ZwlrOutputPowerV1,
        _event: <zwlr_output_power_v1::ZwlrOutputPowerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}
