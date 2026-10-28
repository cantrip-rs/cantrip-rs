use cantrip_api::*;
use cantrip_mechanics::triggers::{NONCREATURE_SPELL_YOU_CAST, PROWESS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell, put a +1/+1 counter on each other creature you control." (CR 122.1, 603.2, 101.4): the
/// creatures its controller controls as it resolves other than Photon, together.
fn counter_on_each_other_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i16)> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature))
        .map(|id| (id, CounterKind::PlusOne, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

/// Photon, Living Light: flying and hexproof are core keywords.
pub static BACK: CardDef = CardDef {
    abilities: &[
        PROWESS,
        triggered(
            NONCREATURE_SPELL_YOU_CAST,
            counter_on_each_other_creature,
            "Whenever you cast a noncreature spell, put a +1/+1 counter on each other creature you control.",
        ),
    ],
    ..oracle::PHOTON_LIVING_LIGHT
};

/// Monica Rambeau: flying is a core keyword. "{2}{R}{W}{W}: Transform Monica Rambeau. Activate only as a sorcery." (CR 701.27a,
/// 712.9): she stays the same object, with Photon's characteristics; an ability of hers does nothing if she has transformed since
/// it was put on the stack (CR 701.27f).
pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        ability(cost("{2}{R}{W}{W}"), effects::transform_self, "{2}{R}{W}{W}: Transform Monica Rambeau. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::MONICA_RAMBEAU
};
