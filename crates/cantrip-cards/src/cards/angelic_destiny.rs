use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, lands, statics, targets, triggers::ATTACHED_CREATURE_DIES_ABOUT_IT};
use cantrip_oracle as oracle;

/// "Return this card to its owner's hand" (CR 400.7f, 603.10a): the Aura has already gone to the graveyard as a state-based
/// action once the creature it enchanted died, so it is the card that became in that graveyard, found from the Aura as it was
/// and the creature (the ability's subject) it left with. It returns only if it is still in that graveyard as the ability
/// resolves: one that has left it since (the 2011-09-22 ruling) stays where it went.
fn return_this_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    if let Some(aura) = g.found_aura_after_enchanted_left(item.object, creature) {
        g.return_to_hand(&[aura]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(ATTACHED_CREATURE_DIES_ABOUT_IT, return_this_card, "When enchanted creature dies, return this card to its owner's hand.")],
    statics: &[
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Angel) }>, "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types."),
        static_ability(Layer::Ability, attached_object, statics::have_flying_and_first_strike, "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.").continues(),
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<4, 4>, "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.").continues(),
    ],
    ..oracle::ANGELIC_DESTINY
};
