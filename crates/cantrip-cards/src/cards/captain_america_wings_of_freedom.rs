use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_ATTACKS};
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// "Whenever Captain America attacks, each other Hero you control gets +X/+X until end of turn, where X is Captain America's
/// toughness." (CR 611.2c, 608.2h): X is worked out once as the ability resolves, from his toughness as the layers have it then
/// (as he last was if he has gone), and the Heroes are those its controller controls then, other than him, the type read through
/// the words of this card's text (CR 612); no Hero that enters later gets it.
fn pump_other_heroes(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.chars(item.object).toughness.unwrap_or_default();
    let hero = g.words(item.object).subtype(Subtype::Hero);
    let heroes: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).has_subtype(hero)).collect();
    instruction(g, |g| {
        for &id in &heroes {
            pump(g, item.object, id, x, x);
        }
    });
    Ok(())
}

/// Flying and first strike are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(
            THIS_ATTACKS,
            pump_other_heroes,
            "Whenever Captain America attacks, each other Hero you control gets +X/+X until end of turn, where X is Captain America's toughness.",
        ),
    ],
    ..oracle::CAPTAIN_AMERICA_WINGS_OF_FREEDOM
};
