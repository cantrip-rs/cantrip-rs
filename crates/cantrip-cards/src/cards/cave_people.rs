use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// Target creature gains the landwalk named by this ability's text (CR 612.2, 702.14).
fn grant_landwalk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0)
        && let Some(walk) = Keyword::landwalk(g.words(item.object).subtype(Subtype::Mountain))
    {
        effects::gain_until_end_of_turn(g, item.object, creature, walk);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, effects::pump_self::<1, -2>, "Whenever this creature attacks, it gets +1/-2 until end of turn."),
        ability(cost("{1}{R}{R}").and(TAP), grant_landwalk, "{1}{R}{R}, {T}: Target creature gains mountainwalk until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::CAVE_PEOPLE
};
