use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, lands, mana};
use cantrip_oracle as oracle;

/// "{2}{W}{B}: This land becomes a 1/4 white and black Nightmare creature until end of turn. It's still a land." (CR 613.1d,
/// 613.4b, 613.1e)
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Black).0);
    lands::becomes_creature_until_end_of_turn(g, item, (1, 4), colors, Subtype::Nightmare, Keywords::NONE);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::White, Color::Black]), "{T}: Add {W} or {B}."),
        ability(cost("{2}{W}{B}"), becomes_creature, "{2}{W}{B}: This land becomes a 1/4 white and black Nightmare creature until end of turn. It's still a land."),
        triggered(THIS_ATTACKS, effects::defending_player_loses_life_you_gain_life::<2>, "Whenever this land attacks, defending player loses 2 life and you gain 2 life."),
    ],
    ..oracle::RESTLESS_FORTRESS
};
