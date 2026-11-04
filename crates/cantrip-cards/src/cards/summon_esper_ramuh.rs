use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "I — Judgment Bolt — This creature deals damage equal to the number of noncreature, nonland cards in your graveyard to
/// target creature an opponent controls." (CR 608.2h, 120.3): the cards are counted as the chapter resolves (ruling
/// 2025-06-06); a card in a graveyard has the types of the card (CR 404.1), so an artifact creature or a land creature is not
/// counted. The damage is dealt by the Saga creature, and not at all if the target is no longer legal (CR 608.2b).
fn judgment_bolt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .filter(|&&id| {
            let chars = g.chars(id);
            !chars.is(CardType::Creature) && !chars.is(CardType::Land)
        })
        .count() as u16;
    damage_target(g, item, 0, n);
    Ok(())
}

/// "II, III — Wizards you control get +1/+0 until end of turn." (CR 611.2c): `effects::subtype_you_control_get` over Wizard.
fn wizards_get_plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::subtype_you_control_get(g, item, Subtype::Wizard, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            judgment_bolt,
            "I \u{2014} Judgment Bolt \u{2014} This creature deals damage equal to the number of noncreature, nonland cards in your graveyard to target creature an opponent controls.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, wizards_get_plus_one, "II \u{2014} Wizards you control get +1/+0 until end of turn."),
        chapter(3, wizards_get_plus_one, "III \u{2014} Wizards you control get +1/+0 until end of turn."),
    ],
    ..oracle::SUMMON_ESPER_RAMUH
};
