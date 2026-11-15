use cantrip_api::*;
use cantrip_mechanics::triggers::ward;
use cantrip_mechanics::{Animation, animate, mana};
use cantrip_oracle as oracle;

/// "Until end of turn, this land becomes a 7/7 blue Giant creature with ward {3}. It's still a land." (CR 613.1d, 613.1f,
/// 613.4b): the color and creature type are the ones its own text has now (CR 612); ward is the ability its delayed
/// list holds, granted for the turn (CR 702.21a).
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    let words = g.words(me);
    let animation = Animation {
        power: 7,
        toughness: 7,
        colors: Some(words.colors(ColorSet::of(Color::Blue))),
        types: TypeSet::NONE,
        subtypes: &[words.subtype(Subtype::Giant)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: Some(AbilityRef::delayed(item.ability.card, 0)),
    };
    animate(g, me, me, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_if_two_or_more_other_lands),
    abilities: &[
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        ability(cost("{5}{U}"), becomes_creature, "{5}{U}: Until end of turn, this land becomes a 7/7 blue Giant creature with ward {3}. It's still a land."),
    ],
    delayed: &[ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)")],
    ..oracle::HALL_OF_STORM_GIANTS
};
