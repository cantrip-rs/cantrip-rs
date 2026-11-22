//! "Modified" (CR 700.9): a permanent with a counter on it, that is equipped, or that is enchanted by an Aura its controller controls.

use cantrip_api::*;

use crate::targets;

/// Whether `id` is a modified permanent (CR 700.9), through the layers: it has one or more counters of any kind on it, an Equipment
/// is attached to it (whoever controls the Equipment), or an Aura controlled by the permanent's controller is attached to it (an
/// Aura an opponent controls does not make it modified). Costume Closet and Silver Sable, Mercenary Leader ask it.
///
/// Asked by a look-back ability ("whenever a modified creature leaves the battlefield", CR 603.10a), `q` is the game as it was just
/// before the event (the core's `BoardThen`): the permanent's counters are its last-known ones, and what was attached to it then is
/// what `permanents_of` and `attached_to` say then - an Equipment or Aura that left with it (a "destroy all nonland permanents")
/// included, one that left earlier not.
pub fn is_modified(q: &dyn Query, id: ObjectId) -> bool {
    let attachments = q.state().seats().flat_map(|p| q.permanents_of(p)).filter(|&a| q.attached_to(a) == Some(id));
    modified(q.obj(id).counter_array(), attachments.map(|a| (q.chars(a), q.controller(a))), q.controller(id))
}

/// `is_modified` for a layer-6 static ability's condition ("has flying as long as it's modified", CR 604.1, 613.1f), where
/// the Equipment and Auras are judged on what the layers before the one being applied made them and the controller is as the
/// layers stand (`LayerQuery::chars`, `LayerQuery::controller`). Skyward Spider asks it.
pub fn is_modified_in_layers(view: &dyn LayerQuery, id: ObjectId) -> bool {
    let attachments = view.state().battlefield().iter().copied().filter(|&a| view.state().obj(a).attached_to == Some(id));
    modified(view.state().obj(id).counter_array(), attachments.map(|a| (view.chars(a), view.controller(a))), view.controller(id))
}

/// CR 700.9: a counter of any kind, an Equipment, or an Aura the permanent's `controller` controls, each attachment given with
/// its characteristics and controller.
fn modified(counters: [i32; CounterKind::COUNT], mut attachments: impl Iterator<Item = (Chars, PlayerId)>, controller: PlayerId) -> bool {
    counters.iter().any(|&n| n > 0)
        || attachments.any(|(chars, by)| chars.has_subtype(Subtype::Equipment) || chars.has_subtype(Subtype::Aura) && by == controller)
}

fn modified_creature_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::on_battlefield(q, id)
        && q.chars(id).is(CardType::Creature)
        && q.controller(id) == asker.controller
        && is_modified(q, id))
}

/// "Target modified creature you control" (CR 115.1, 700.9). Silver Sable, Mercenary Leader prints it.
pub const MODIFIED_CREATURE_YOU_CONTROL: TargetSpec = target(modified_creature_you_control_filter);
