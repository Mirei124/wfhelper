use std::env::var;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use wayland_client::{
    delegate_noop,
    protocol::{wl_output, wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};
use wayland_protocols_plasma::{
    dpms::client::{org_kde_kwin_dpms, org_kde_kwin_dpms_manager},
    idle::client::{org_kde_kwin_idle, org_kde_kwin_idle_timeout},
};
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1, zwlr_output_power_v1,
};

// which creates a notification after inactive for a while
trait IdleNotifier {
    fn get_idle_notify(
        &self,
        seat: &wl_seat::WlSeat,
        wait_time: u32,
        queue_handle: &QueueHandle<State>,
    ) -> Box<dyn IdleNotification>;
}

// created by IdleNotifier
trait IdleNotification {
    fn destroy_notifier(self: Box<Self>);
}

trait OutputPowerManager {
    fn set_output_off(&self, state: &State, qhandle: &QueueHandle<State>);
    fn set_output_on(&self, state: &State, qhandle: &QueueHandle<State>);
}

impl IdleNotifier for ext_idle_notifier_v1::ExtIdleNotifierV1 {
    fn get_idle_notify(
        &self,
        seat: &wl_seat::WlSeat,
        wait_time: u32,
        queue_handle: &QueueHandle<State>,
    ) -> Box<dyn IdleNotification> {
        let notification = self.get_idle_notification(wait_time, seat, queue_handle, ());
        Box::new(notification)
    }
}

impl IdleNotification for ext_idle_notification_v1::ExtIdleNotificationV1 {
    fn destroy_notifier(self: Box<Self>) {
        self.destroy();
    }
}

impl IdleNotifier for org_kde_kwin_idle::OrgKdeKwinIdle {
    fn get_idle_notify(
        &self,
        seat: &wl_seat::WlSeat,
        wait_time: u32,
        queue_handle: &QueueHandle<State>,
    ) -> Box<dyn IdleNotification> {
        let notification = self.get_idle_timeout(seat, wait_time, queue_handle, ());
        Box::new(notification)
    }
}

impl IdleNotification for org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout {
    fn destroy_notifier(self: Box<Self>) {
        self.release();
    }
}

impl OutputPowerManager for zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1 {
    fn set_output_off(&self, state: &State, qhandle: &QueueHandle<State>) {
        for output in &state.outputs {
            let output_power = self.get_output_power(output, qhandle, ());
            output_power.set_mode(zwlr_output_power_v1::Mode::Off);
            output_power.destroy();
        }
    }

    fn set_output_on(&self, state: &State, qhandle: &QueueHandle<State>) {
        for output in &state.outputs {
            let output_power = self.get_output_power(output, qhandle, ());
            output_power.set_mode(zwlr_output_power_v1::Mode::On);
            output_power.destroy();
        }
    }
}

impl OutputPowerManager for org_kde_kwin_dpms_manager::OrgKdeKwinDpmsManager {
    fn set_output_off(&self, state: &State, qhandle: &QueueHandle<State>) {
        for output in &state.outputs {
            let dpms = self.get(output, qhandle, ());
            dpms.set(org_kde_kwin_dpms::Mode::Off as u32);
            dpms.release();
        }
    }

    fn set_output_on(&self, state: &State, qhandle: &QueueHandle<State>) {
        for output in &state.outputs {
            let dpms = self.get(output, qhandle, ());
            dpms.set(org_kde_kwin_dpms::Mode::On as u32);
            dpms.release();
        }
    }
}

struct State {
    idle_notifier: Option<Box<dyn IdleNotifier>>,
    seat: Option<wl_seat::WlSeat>,
    outputs: Vec<wl_output::WlOutput>,
    manager: Option<Box<dyn OutputPowerManager>>,
    resumed: bool,
}

pub fn do_screen_off(wait_time: Option<u32>) {
    // millisec
    let wait_time = wait_time.unwrap_or(500);
    let session_type = var("XDG_SESSION_TYPE").unwrap();
    match &session_type[..] {
        "wayland" => wayland_screen_off(wait_time),
        "x11" => x11_screen_off(wait_time as u64),
        _ => {
            unreachable!("No match for XDG_SESSION_TYPE: {session_type}");
        }
    }
}

fn x11_screen_off(wait_time: u64) {
    sleep(Duration::from_millis(wait_time));
    Command::new("xset")
        .args(["dpms", "force", "standby"])
        .status()
        .unwrap();
}

fn wayland_screen_off(wait_time: u32) {
    let connection = Connection::connect_to_env().unwrap();
    let display = connection.display();
    let mut event_queue = connection.new_event_queue();
    let queue_handle = event_queue.handle();

    let mut state = State {
        idle_notifier: None,
        seat: None,
        outputs: Vec::new(),
        manager: None,
        resumed: false,
    };

    display.get_registry(&queue_handle, ());
    event_queue.blocking_dispatch(&mut state).unwrap();

    if state.idle_notifier.is_none() {
        panic!("Current wayland compositor supports neither ext_idle_notifier_v1 nor org_kde_kwin_idle");
    }

    if state.manager.is_none() {
        panic!("Current wayland compositor supports neither zwlr_output_power_manager_v1 nor org_kde_kwin_dpms_manager");
    }

    // set timeout
    let notification = state.idle_notifier.as_ref().unwrap().get_idle_notify(
        state.seat.as_ref().expect("WlSeat is None"),
        wait_time,
        &queue_handle,
    );

    while !state.resumed {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }

    // cancel timeout
    notification.destroy_notifier();

    event_queue.roundtrip(&mut state).unwrap();
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
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
                "wl_output" => {
                    let output =
                        proxy.bind::<wl_output::WlOutput, _, _>(name, version, qhandle, ());
                    state.outputs.push(output);
                }
                "wl_seat" => {
                    let seat = proxy.bind::<wl_seat::WlSeat, _, _>(name, version, qhandle, ());
                    state.seat = Some(seat);
                }
                "ext_idle_notifier_v1" => {
                    let notifier = proxy.bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.idle_notifier = Some(Box::new(notifier));
                }
                "zwlr_output_power_manager_v1" => {
                    let manager = proxy
                        .bind::<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, _, _>(
                            name,
                            version,
                            qhandle,
                            (),
                        );
                    state.manager = Some(Box::new(manager));
                }
                "org_kde_kwin_idle" => {
                    let notifier = proxy.bind::<org_kde_kwin_idle::OrgKdeKwinIdle, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.idle_notifier = Some(Box::new(notifier));
                }
                "org_kde_kwin_dpms_manager" => {
                    let manager = proxy
                        .bind::<org_kde_kwin_dpms_manager::OrgKdeKwinDpmsManager, _, _>(
                            name,
                            version,
                            qhandle,
                            (),
                        );
                    state.manager = Some(Box::new(manager));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: <ext_idle_notification_v1::ExtIdleNotificationV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                state
                    .manager
                    .as_ref()
                    .unwrap()
                    .set_output_off(state, qhandle);
            }
            ext_idle_notification_v1::Event::Resumed => {
                state
                    .manager
                    .as_ref()
                    .unwrap()
                    .set_output_on(state, qhandle);
                state.resumed = true;
            }
            _ => {
                unreachable!("{event:?}");
            }
        }
    }
}

impl Dispatch<org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout,
        event: <org_kde_kwin_idle_timeout::OrgKdeKwinIdleTimeout as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            org_kde_kwin_idle_timeout::Event::Idle => {
                state
                    .manager
                    .as_ref()
                    .unwrap()
                    .set_output_off(state, qhandle);
            }
            org_kde_kwin_idle_timeout::Event::Resumed => {
                state
                    .manager
                    .as_ref()
                    .unwrap()
                    .set_output_on(state, qhandle);
                state.resumed = true;
            }
            _ => {
                unreachable!("{event:?}");
            }
        }
    }
}

delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ignore wl_output::WlOutput);
delegate_noop!(State: ignore ext_idle_notifier_v1::ExtIdleNotifierV1);
delegate_noop!(State: ignore zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1);
delegate_noop!(State: ignore zwlr_output_power_v1::ZwlrOutputPowerV1);
delegate_noop!(State: ignore org_kde_kwin_idle::OrgKdeKwinIdle);
delegate_noop!(State: ignore org_kde_kwin_dpms_manager::OrgKdeKwinDpmsManager);
delegate_noop!(State: ignore org_kde_kwin_dpms::OrgKdeKwinDpms);
