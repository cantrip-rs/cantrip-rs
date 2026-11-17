use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::tokens::{self, TREASURE};
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Create a Treasure token for each creature that crewed it this turn" (CR 702.122c, 111.10a): one Treasure for each creature
/// was tapped to pay the cost of this Vehicle's crew ability this turn, a creature that has since left the battlefield still counting;
/// no Treasure if nothing crewed it (it can attack because it was crewed earlier and the history of the turn is empty only when
/// a card other than crew made it a creature).
fn treasure_for_each_crewer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let crewers = g.history().crewed(item.object).count() as u32;
    tokens::create_by_rule(g, item.controller, &TREASURE, crewers, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            treasure_for_each_crewer,
            "Whenever this Vehicle attacks, create a Treasure token for each creature that crewed it this turn. (They're artifacts with \"{T}, Sacrifice this token: Add one mana of any color.\")",
        ),
        crew(1, "Crew 1. Activate only once each turn. (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)").once_each_turn(),
    ],
    ..oracle::LUXURIOUS_LOCOMOTIVE
};

pub static EXTRA: &[&CardDef] = &[&TREASURE];
