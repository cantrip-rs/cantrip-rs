use cantrip_api::*;
use cantrip_mechanics::{effects, lands, mana, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land."
/// (CR 613.1d, 613.4b, 613.1e)
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = ColorSet(ColorSet::of(Color::Black).0 | ColorSet::of(Color::Red).0);
    lands::becomes_creature_until_end_of_turn(g, item, (2, 3), colors, Subtype::Insect, Keywords::of(&[Keyword::Menace]));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Black, Color::Red]), "{T}: Add {B} or {R}."),
        ability(cost("{1}{B}{R}"), becomes_creature, "{1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land."),
        triggered(
            triggers::THIS_ATTACKS,
            |g, item| effects::may_discard_to_draw(g, item, 1),
            "Whenever this land attacks, you may discard a card. If you do, draw a card.",
        ),
    ],
    ..oracle::RESTLESS_VENTS
};
