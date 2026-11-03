use cantrip_api::*;
use cantrip_mechanics::{mana, targets};
use cantrip_oracle as oracle;

/// The enchanted land's controller adds {G} (CR 605.1b).
fn add_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(land) = item.subject {
        g.add_mana(g.controller(land), Mana::G, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::TAPPED, mana::enchanted_permanent_tapped_for_mana),
        add_green,
        "Whenever enchanted land is tapped for mana, its controller adds an additional {G}.",
    ).mana_ability(TriggeredMana::One(Mana::G))],
    enchant: Some(targets::LAND),
    ..oracle::WILD_GROWTH
};
