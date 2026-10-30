use cantrip_api::*;
use cantrip_mechanics::effects::gain_hexproof_and_indestructible_until_end_of_turn as protect;
use cantrip_mechanics::{chosen_type, triggers};
use cantrip_oracle as oracle;

/// "Choose a creature type. Other permanents you control of that type gain hexproof and indestructible until end of turn."
/// The type is chosen as the ability resolves (CR 608.2d), then the permanents are found (CR 608.2c): every permanent its
/// controller has with that subtype, this creature apart.
fn protect_the_chosen_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = chosen_type::creature_type_now(g, you, item.object)?;
    let matching: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| id != item.object && g.chars(id).has_subtype(chosen)).collect();
    instruction(g, |g| {
        for id in matching {
            protect(g, item.object, id);
        }
    });
    Ok(())
}

/// Convoke is a characteristic of the card, read from its JSON; Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        protect_the_chosen_type,
        "When this creature enters, choose a creature type. Other permanents you control of that type gain hexproof and indestructible until end of turn.",
    )],
    ..oracle::SELFLESS_SAFEWRIGHT
};
