//! Keep presence authoritative and preserve detailed data only for still-present identities.
use crate::{
    fields::Field,
    model::{Device, Snapshot},
};
fn cached(old: &[Device], mut fresh: Device) -> Device {
    if let Some(previous) = old.iter().find(|d| d.key == fresh.key) {
        let parent = |d: &Device| {
            d.fields
                .iter()
                .find(|f| f.id == "node.parent_key")
                .map(|f| f.value.clone())
        };
        if parent(previous) != parent(&fresh) {
            return fresh;
        }
        let mut d = previous.clone();
        d.connection_status = fresh.connection_status;
        d.windows_problem = fresh.windows_problem;
        d.fields.retain(|f| f.id != "node.parent_key");
        d.fields
            .extend(fresh.fields.drain(..).filter(|f| f.id == "node.parent_key"));
        d
    } else {
        fresh
    }
}
pub fn merge_presence(old: Option<&Snapshot>, mut fast: Snapshot) -> Snapshot {
    let Some(old) = old else {
        return fast;
    };
    fast.devices = fast
        .devices
        .into_iter()
        .map(|d| cached(&old.devices, d))
        .collect();
    fast.topology = fast
        .topology
        .into_iter()
        .map(|d| cached(&old.topology, d))
        .collect();
    if !fast.inventory_complete {
        for d in &old.devices {
            if !fast.devices.iter().any(|n| n.key == d.key) {
                fast.devices.push(d.clone());
            }
        }
        for d in &old.topology {
            if !fast.topology.iter().any(|n| n.key == d.key) {
                fast.topology.push(d.clone());
            }
        }
    } else {
        // Failed/unmapped ports have no present device interface. Only a full hub pass
        // can authoritatively remove these fallback identities.
        for d in &old.devices {
            if d.key.contains(":port:") && !fast.devices.iter().any(|n| n.key == d.key) {
                fast.devices.push(d.clone());
            }
        }
    }
    fast
}
pub fn apply_details(live: &Snapshot, full: Snapshot) -> Snapshot {
    let mut out = live.clone();
    out.issues = full.issues;
    if full.inventory_complete {
        out.devices
            .retain(|d| !d.key.contains(":port:") || full.devices.iter().any(|n| n.key == d.key));
    }
    for d in &full.devices {
        if (d.key.contains(":port:") || !live.inventory_complete)
            && !out.devices.iter().any(|n| n.key == d.key)
        {
            out.devices.push(d.clone());
        }
    }

    for (dest, source) in [
        (&mut out.devices, &full.devices),
        (&mut out.topology, &full.topology),
    ] {
        for d in dest {
            let found = source.iter().find(|n| n.key == d.key);
            if let Some(fresh) = found {
                let parent = d.fields.iter().find(|f| f.id == "node.parent_key").cloned();
                *d = fresh.clone();
                if !d.fields.iter().any(|f| f.id == "node.parent_key")
                    && let Some(p) = parent
                {
                    d.fields.push(p);
                }
            } else if d.fields.iter().any(|f| f.id == "live.details_pending") {
                d.fields.retain(|f| f.id != "live.details_pending");
                d.fields.push(Field::new(
                    "live.details_pending",
                    "Подробности",
                    "Windows",
                    "Не получены; повторите запрос",
                    "Full scan",
                    false,
                ));
            }
        }
    }
    out.captured_unix_ms = full.captured_unix_ms;
    out
}
pub fn same_presence(a: &Snapshot, b: &Snapshot) -> bool {
    let keys = |s: &Snapshot| {
        let mut k: Vec<_> = s
            .devices
            .iter()
            .chain(&s.topology)
            .filter(|d| !d.key.contains(":port:"))
            .map(|d| {
                (
                    d.key.clone(),
                    d.windows_problem,
                    d.connection_status,
                    d.fields
                        .iter()
                        .find(|f| f.id == "node.parent_key")
                        .map(|f| f.value.clone()),
                )
            })
            .collect();
        k.sort();
        k
    };
    keys(a) == keys(b)
}
