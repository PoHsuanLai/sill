//! Spawning any [`Surface`]: one surface per slot per output, kept alive across host events. A
//! new output's or slot's surface is spawned (hidden or shown, as `Surface::start` says), one
//! whose slot the settings dropped is closed, and one whose layer size changed is resized.

use shell_host::{HostCtx, HostRoot, LayerConfig, LayerOp, OutputInfo};
use sill_ui_kit::surface::{MountProps, OutputPolicy, OutputRank, Place, Start, Surface, mount};

use crate::daemon::msg::ShellMsg;
use crate::daemon::surfaces::{Spawn, spawn_hidden, surface_root};
use crate::outputs::{FocusRoute, OutputSurfaces, Outputs, Spawned};

/// Spawn, resize and close `S`'s surfaces on every output in `outputs` that `S::OUTPUTS` wants.
/// The surfaces are given `S::link` as a root context, and the other links `S::needs` names.
pub fn spawn_surface<S: Surface>(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    spawn: &Spawn,
) {
    let file = spawn.settings.borrow().clone();
    let link = S::link(spawn.links);
    let main = outputs.pick(OutputPolicy::Main, None);
    for (index, (info, row)) in outputs.outputs.iter_mut().enumerate() {
        let place = Place {
            output: info,
            rank: OutputRank::of_index(index),
            settings: &file,
        };
        // A surface that lives on the main display only is closed where it is not wanted any more
        // (the main display moved), like a slot the settings dropped.
        let wanted = match S::OUTPUTS.spawns_on(info.id, main) {
            true => S::slots(&place),
            false => Vec::new(),
        };
        close_dropped::<S>(ctx, row, &wanted);
        for slot in wanted {
            let config = S::layer(info, &file, slot);
            match held::<S>(row, slot) {
                Some(surface) => resize(ctx, surface, &config),
                None => spawn_slot::<S>(ctx, row, info, spawn, &link, slot, config),
            }
        }
    }
}

/// `slot`'s key in a row, which holds every kind's slots at once.
fn slot_key<S: Surface>(slot: S::Slot) -> String {
    format!("{slot:?}")
}

/// The row's surface of `S` in `slot`, if spawned.
fn held<S: Surface>(row: &mut OutputSurfaces, slot: S::Slot) -> Option<&mut Spawned> {
    let key = slot_key::<S>(slot);
    row.spawned
        .iter_mut()
        .find(|surface| surface.name == S::NAME && surface.slot == key)
}

/// Close the row's surfaces of `S` whose slot is not in `wanted`.
fn close_dropped<S: Surface>(
    ctx: &mut HostCtx<ShellMsg>,
    row: &mut OutputSurfaces,
    wanted: &[S::Slot],
) {
    let keys: Vec<String> = wanted.iter().map(|slot| slot_key::<S>(*slot)).collect();
    row.spawned.retain(|surface| {
        let dropped = surface.name == S::NAME && !keys.contains(&surface.slot);
        if dropped {
            ctx.close(surface.id);
        }
        !dropped
    });
}

/// Ask the compositor for `config`'s size when it is not the one last asked for.
fn resize(ctx: &mut HostCtx<ShellMsg>, surface: &mut Spawned, config: &LayerConfig) {
    if surface.size != config.size {
        ctx.apply(surface.id, LayerOp::SetSize(config.size));
        surface.size = config.size;
    }
}

/// Spawn `slot` of `S` on `info` and record it in `row`.
fn spawn_slot<S: Surface>(
    ctx: &mut HostCtx<ShellMsg>,
    row: &mut OutputSurfaces,
    info: &OutputInfo,
    spawn: &Spawn,
    link: &S::Link,
    slot: S::Slot,
    config: LayerConfig,
) {
    let props = MountProps::<S>::new(info.clone(), slot);
    let root = S::needs(
        surface_root(
            HostRoot::with_props(mount::<S>, props),
            spawn.services,
            spawn.settings_feed,
        ),
        spawn.links,
    )
    .context(link.clone());
    let namespace = config.namespace.clone();
    let size = config.size;
    let Some(id) = spawn_layer(ctx, config, root, info, S::start()) else {
        return;
    };
    row.spawned.push(Spawned {
        name: S::NAME,
        slot: slot_key::<S>(slot),
        id,
        namespace,
        size,
        role: S::ROLE,
        focus: FocusRoute::of::<S>(link, info.id),
    });
}

/// Spawn a layer surface, shown or hidden before its first paint; an error is logged.
fn spawn_layer(
    ctx: &mut HostCtx<ShellMsg>,
    config: LayerConfig,
    root: HostRoot,
    info: &OutputInfo,
    start: Start,
) -> Option<shell_host::SurfaceId> {
    match start {
        Start::Hidden => spawn_hidden(ctx, config, root, info),
        Start::Shown => {
            let namespace = config.namespace.clone();
            ctx.spawn_layer(config, root)
                .inspect_err(|error| {
                    eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name);
                })
                .ok()
        }
    }
}
