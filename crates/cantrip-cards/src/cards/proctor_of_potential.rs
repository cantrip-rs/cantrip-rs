use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature you control enters, surveil 1" (CR 603.6a, 701.25): this creature itself
/// included.
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// "Activate only if you've scried or surveilled this turn" (CR 602.5b), read when the ability is activated.
fn scried_or_surveilled_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    cantrip_mechanics::history::scried_or_surveilled_this_turn(q, asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_YOU_CONTROL_ENTERS,
            surveil_one,
            "Whenever this creature or another creature you control enters, surveil 1.",
        ),
        ability(
            cost("{W}{U}"),
            cantrip_mechanics::effects::return_this_card_with_finality,
            "{W}{U}: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only if you've scried or surveilled this turn.",
        )
        .from(ZoneKind::Graveyard)
        .condition(scried_or_surveilled_this_turn),
    ],
    ..oracle::PROCTOR_OF_POTENTIAL
};
