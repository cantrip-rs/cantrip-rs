use cantrip_api::*;
use cantrip_mechanics::{effects, saddle, triggers};
use cantrip_oracle as oracle;

/// "It gains indestructible until end of turn. Untap all creatures you control." (CR 611.2a, 701.26b): the next sentence
/// (CR 608.2c) untaps every creature you control as it resolves, whether or not the Mount is still around to gain anything.
fn gains_indestructible_and_untap_all(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Indestructible);
    g.end_sentence();
    effects::untap_all_creatures_you_control(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            gains_indestructible_and_untap_all,
            "Whenever this creature attacks while saddled, it gains indestructible until end of turn. Untap all creatures you control.",
        ),
        saddle::ability(4, "Saddle 4 (Tap any number of other creatures you control with total power 4 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::UNSWERVING_SLOTH
};
