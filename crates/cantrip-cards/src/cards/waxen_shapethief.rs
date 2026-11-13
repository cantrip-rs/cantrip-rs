use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_mechanics::triggers::cycling;
use cantrip_oracle as oracle;

/// "You may have this creature enter as a copy of an artifact or creature you control." A replacement effect on its entering
/// (CR 614.1c, 707.2): a choice, not a target (CR 608.2d), made by the player it enters under (CR 614.12). With nothing of
/// yours to copy nothing is asked, and declining or having nothing leaves it the printed 0/0 that state-based actions put
/// into the graveyard (CR 704.5f). It copies with no exception.
fn enter_as_copy(g: &mut dyn Game, entry: &mut Entering) -> Result<(), Suspend> {
    copy::enter_as_copy_of_one_you_control(g, entry, |c| c.is(CardType::Artifact) || c.is(CardType::Creature), |_, copy| copy)
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(enter_as_copy),
    abilities: &[cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::WAXEN_SHAPETHIEF
};
