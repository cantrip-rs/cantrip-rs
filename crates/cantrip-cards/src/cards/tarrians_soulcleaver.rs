use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::have_vigilance;
use cantrip_oracle as oracle;

/// "Whenever another artifact or creature is put into a graveyard from the battlefield" (CR 603.6c, 603.10a): any artifact or
/// creature but this Equipment, anyone's, token or card, looked back on as it last existed on the battlefield. Each one is its own
/// event, so several at once trigger as many times (CR 603.2c); the Equipment leaving in the same event still sees them.
fn another_artifact_or_creature_put_into_a_graveyard_from_the_battlefield(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && (q.chars(*old).is(CardType::Artifact) || q.chars(*old).is(CardType::Creature)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::ZONE_CHANGE, another_artifact_or_creature_put_into_a_graveyard_from_the_battlefield),
            effects::plus_one_counter_on_equipped_creature,
            "Whenever another artifact or creature is put into a graveyard from the battlefield, put a +1/+1 counter on equipped creature.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance.")],
    ..oracle::TARRIANS_SOULCLEAVER
};
