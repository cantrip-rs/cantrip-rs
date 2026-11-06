use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_FOR_THE_FIRST_TIME_EACH_TURN;
use cantrip_oracle as oracle;

/// Untap all creatures you control (CR 701.26a). After this phase, there is an additional combat phase (CR 500.8).
fn untap_and_extra_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.untap_together(&creatures);
    g.add_combat_phase();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS_FOR_THE_FIRST_TIME_EACH_TURN,
        untap_and_extra_combat,
        "Whenever Aurelia attacks for the first time each turn, untap all creatures you control. After this phase, there is an additional combat phase.",
    )],
    ..oracle::AURELIA_THE_WARLEADER
};
