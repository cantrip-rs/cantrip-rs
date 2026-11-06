use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::have_trample;
use cantrip_mechanics::{chosen_type, tokens};
use cantrip_oracle as oracle;

/// "Choose a creature type. Create a Treasure token for each creature you control of that type": the type is chosen
/// as the ability resolves (CR 608.2d), the creatures counted after, and the Treasures created together (CR 101.4).
fn treasures_for_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = chosen_type::creature_type_now(g, you, item.object)?;
    let count = g
        .permanents_of(you)
        .into_iter()
        .filter(|&id| {
            let c = g.chars(id);
            c.is(CardType::Creature) && c.has_subtype(chosen)
        })
        .count();
    if count == 0 {
        return Ok(());
    }
    let treasure = g.card_id(&tokens::TREASURE);
    let all: Vec<(CardId, PlayerId)> = (0..count).map(|_| (treasure, you)).collect();
    g.create_tokens_together(&all)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, treasures_for_type, "Whenever equipped creature deals combat damage to a player, choose a creature type. Create a Treasure token for each creature you control of that type."),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<2, 2>, "Equipped creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_trample, "Equipped creature has trample."),
    ],
    ..oracle::ORCRIST_GOBLIN_CLEAVER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
