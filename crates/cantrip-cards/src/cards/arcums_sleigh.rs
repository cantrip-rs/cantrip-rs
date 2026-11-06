use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}, {T}: Target creature gains vigilance until end of turn. Activate only during combat and only if defending player controls a snow land." (CR 608.2, 611.2a).
fn target_creature_gains_vigilance_activate(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
    }
    Ok(())
}

/// "{2}, {T}: Target creature gains vigilance until end of turn. Activate only during combat and only if defending player controls a snow land." (CR 602.5).
fn target_creature_gains_vigilance_activate_if(q: &dyn Query, _: &Asker) -> bool {
    ((Step::BeginCombat as u8)..=(Step::EndOfCombat as u8))
        .contains(&(q.state().turn.step() as u8))
        && q
            .state()
            .opponents(q.state().turn.active)
            .into_iter()
            .any(|defending: PlayerId| {
                q
                    .state()
                    .battlefield()
                    .iter()
                    .copied()
                    .filter(|&id| {
                        q.controller(id) == defending
                            && (q.chars(id).supertypes.snow
                                && q.chars(id).is(CardType::Land))
                    })
                    .count() >= 1
            })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                target_creature_gains_vigilance_activate,
                "{2}, {T}: Target creature gains vigilance until end of turn. Activate only during combat and only if defending player controls a snow land.",
            )
            .targets(&[targets::CREATURE])
            .condition(target_creature_gains_vigilance_activate_if),
    ],
    ..oracle::ARCUMS_SLEIGH
};
