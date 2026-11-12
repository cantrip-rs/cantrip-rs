use cantrip_api::*;
use cantrip_mechanics::effects::{plus_one_counter_on_this, untap_self};
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while saddled, untap it and put a +1/+1 counter on it." (CR 701.26b, 122.1a): two instructions, each an event of its own,
/// the counter going on whether or not the untap did anything (a Mustang that attacks with vigilance is not tapped to begin with).
fn untap_it_and_put_a_counter_on_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    untap_self(g, item)?;
    plus_one_counter_on_this(g, item)?;
    Ok(())
}

/// "Saddle 1" (CR 702.171a) is the shared mechanic; "attacks while saddled" is judged when the Mustang attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            untap_it_and_put_a_counter_on_it,
            "Whenever this creature attacks while saddled, untap it and put a +1/+1 counter on it.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BRIGHTFIELD_MUSTANG
};
