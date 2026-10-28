use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Return up to one target creature card from your graveyard to your hand. Amass Goblins 3." (CR 400.3, 701.47a): with no
/// target chosen the Army is still made; if the one chosen became illegal the spell does not resolve at all (CR 608.2b), the
/// core's.
fn return_and_amass(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    g.end_sentence();
    amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, 3)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.optional()], return_and_amass),
    ..oracle::GATHERING_OF_DARKNESS
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
