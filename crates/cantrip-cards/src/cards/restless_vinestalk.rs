use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Blue).0);
    lands::becomes_creature_until_end_of_turn(g, item, (5, 5), colors, Subtype::Plant, Keywords::of(&[Keyword::Trample]));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Green, Color::Blue]), "{T}: Add {G} or {U}."),
        ability(cost("{3}{U}{G}"), becomes_creature, "{3}{G}{U}: Until end of turn, this land becomes a 5/5 green and blue Plant creature with trample. It's still a land."),
        triggered(triggers::THIS_ATTACKS, effects::set_first_target_base_pt::<3, 3>, "Whenever this land attacks, up to one other target creature has base power and toughness 3/3 until end of turn.")
            .targets(&[up_to_one(targets::other_creature_filter)]),
    ],
    ..oracle::RESTLESS_VINESTALK
};
