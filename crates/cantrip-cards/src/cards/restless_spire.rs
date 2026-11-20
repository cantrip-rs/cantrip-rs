use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, lands, mana, statics};
use cantrip_oracle as oracle;

/// "{U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature with 'During your turn, this creature has
/// first strike.' It's still a land." (CR 613.1d, 613.4b, 613.1e, 613.1f): the quoted static ability is one of the card's
/// own, given to the land for the turn (`CardDef::delayed`).
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Red).0);
    let first_strike = Some(AbilityRef::delayed(item.ability.card, 0));
    lands::becomes_creature_with_ability_until_end_of_turn(g, item, (2, 1), colors, Subtype::Elemental, Keywords::NONE, first_strike);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Blue, Color::Red]), "{T}: Add {U} or {R}."),
        ability(
            cost("{U}{R}"),
            becomes_creature,
            "{U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature with \"During your turn, this creature has first strike.\" It's still a land.",
        ),
        triggered(THIS_ATTACKS, |g, item| effects::scry(g, item, 1), "Whenever this land attacks, scry 1."),
    ],
    delayed: &[granted_static_ability(
        Layer::Ability,
        statics::itself_during_your_turn,
        statics::have_first_strike,
        "During your turn, this creature has first strike.",
    )],
    ..oracle::RESTLESS_SPIRE
};
