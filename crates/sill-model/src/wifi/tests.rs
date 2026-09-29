use super::{
    AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiEvent, listed_networks, step,
};
use crate::units::{Percent, Presence};

fn ap(ssid: &str, strength: u16, association: Association) -> AccessPoint {
    AccessPoint {
        ssid: Ssid(ssid.to_owned()),
        strength: Percent(strength),
        security: Security::Secured,
        association,
    }
}

fn on(access_points: Vec<AccessPoint>, saved: &[&str]) -> Wifi {
    Wifi {
        device: Presence::Present,
        radio: Radio::On,
        access_points,
        saved: saved.iter().map(|s| Ssid((*s).to_owned())).collect(),
    }
}

#[test]
fn one_row_per_network_connected_then_saved_then_strongest() {
    use Association::{Connected, Idle};
    let wifi = on(
        vec![
            ap("cafe", 90, Idle),
            ap("home", 40, Idle),
            ap("home", 60, Connected),
            ap("office", 30, Idle),
            ap("", 99, Idle),
            ap("attic", 90, Idle),
        ],
        &["office"],
    );
    let rows: Vec<(String, u16, Association, Saved)> = listed_networks(&wifi)
        .into_iter()
        .map(|(ap, saved)| (ap.ssid.0, ap.strength.0, ap.association, saved))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("home".to_owned(), 60, Connected, Saved::New),
            ("office".to_owned(), 30, Idle, Saved::Known),
            ("attic".to_owned(), 90, Idle, Saved::New),
            ("cafe".to_owned(), 90, Idle, Saved::New),
        ]
    );
}

#[test]
fn events_reduce_to_the_expected_wifi() {
    let seen = || vec![ap("home", 60, Association::Connected)];
    let cases: Vec<(&str, Wifi, WifiEvent, Wifi)> = vec![
        (
            "a scan lands",
            on(Vec::new(), &[]),
            WifiEvent::AccessPoints(seen()),
            on(seen(), &[]),
        ),
        (
            "the radio goes off and the list with it",
            on(seen(), &["home"]),
            WifiEvent::Radio(Radio::Off),
            Wifi {
                radio: Radio::Off,
                ..on(Vec::new(), &["home"])
            },
        ),
        (
            "saved networks",
            on(seen(), &[]),
            WifiEvent::Saved(vec![Ssid("home".to_owned())]),
            on(seen(), &["home"]),
        ),
        (
            "networkmanager leaves",
            on(seen(), &["home"]),
            WifiEvent::Gone,
            Wifi::default(),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}
