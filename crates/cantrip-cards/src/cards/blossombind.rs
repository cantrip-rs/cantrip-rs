use cantrip_api::*;
use cantrip_mechanics::{statics, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Aura enters, tap enchanted creature": the creature the Aura is attached to as the trigger resolves, or, if it has left
/// the battlefield by then, the one it was attached to as it left (CR 303.4m, 608.2h).
fn tap_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        g.tap(creature);
    }
    Ok(())
}

/// "Enchanted creature ... can't have counters put on it" (CR 122.6, 101.2) as a `CardDef::counter_bans` entry: counters of any
/// kind, put by an effect, as a cost (blight) or by damage from a source with wither or infect. The counters already on it stay.
fn enchanted_creature_cant_have_counters(q: &dyn Query, asker: &Asker, object: ObjectId, _: PlayerId, _: CounterKind) -> bool {
    q.obj(asker.source).attached_to == Some(object)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    // "Enchanted creature can't become untapped" (CR 502.3, 101.2).
    untap_bans: &[statics::attached_permanent_doesnt_untap],
    counter_bans: &[enchanted_creature_cant_have_counters],
    ..oracle::BLOSSOMBIND
};
