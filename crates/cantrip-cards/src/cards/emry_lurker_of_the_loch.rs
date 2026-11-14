use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::affinity;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn mill_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 4);
    Ok(())
}


pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    abilities: &[
        triggered(THIS_ENTERS, mill_four, "When Emry, Lurker of the Loch enters, mill four cards."),
        ability(TAP, effects::may_cast_first_target_this_turn, "{T}: Choose target artifact card in your graveyard. You may cast that card this turn.")
            .targets(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::EMRY_LURKER_OF_THE_LOCH
};
