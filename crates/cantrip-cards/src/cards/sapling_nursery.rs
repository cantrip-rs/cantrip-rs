use cantrip_api::*;
use cantrip_mechanics::{affinity, lands, effects::gain_indestructible_until_end_of_turn, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 3/4 green Treefolk creature token with reach" (CR 111.1): one for each land that enters, since each is its own
/// trigger (CR 603.2).
fn create_treefolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::TREEFOLK, 1)?;
    Ok(())
}

/// "Treefolk and Forests you control gain indestructible until end of turn" (CR 611.2c): the permanents of the ability's
/// controller that are Treefolk or Forests as it resolves, and no permanent that comes later. The cost exiled the enchantment
/// (CR 602.2b), which is the effect's source as it last was (CR 113.7a). Treefolk and Forest are words of this card's rules
/// text (CR 612), and a permanent is one when the layers made it that type.
fn protect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let (treefolk, forest) = (words.subtype(Subtype::Treefolk), words.subtype(Subtype::Forest));
    let permanents: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&permanent| {
            let subtypes = g.chars(permanent).subtypes;
            subtypes.has(treefolk) || subtypes.has(forest)
        })
        .collect();
    instruction(g, |g| {
        for permanent in permanents {
            gain_indestructible_until_end_of_turn(g, item.object, permanent);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_land_type::<{ lands::sub(Subtype::Forest) }>],
    abilities: &[
        triggered(when_asked(EventMask::ZONE_CHANGE, triggers::landfall), create_treefolk, "Landfall \u{2014} Whenever a land you control enters, create a 3/4 green Treefolk creature token with reach."),
        ability(cost("{1}{G}").and(EXILE_THIS), protect, "{1}{G}, Exile this enchantment: Treefolk and Forests you control gain indestructible until end of turn."),
    ],
    ..oracle::SAPLING_NURSERY
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREEFOLK];
