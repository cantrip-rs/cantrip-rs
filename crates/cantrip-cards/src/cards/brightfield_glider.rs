use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while saddled, it gets +1/+2 and gains flying until end of turn." (CR 611.2a, 613.4c, 702.9a):
/// one instruction of two continuous effects from the Glider, which do nothing once it has left the battlefield (CR 400.7).
fn gets_plus_one_plus_two_and_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        cantrip_mechanics::pump(g, item.object, item.object, 1, 2);
        if g.is_on_battlefield(item.object) {
            gain_until_end_of_turn(g, item.object, item.object, Keyword::Flying);
        }
        Ok(())
    })
}

/// "Vigilance" comes with the generated keywords. "Saddle 3" (CR 702.171a) is the shared mechanic; "attacks while saddled" is
/// judged when the Glider attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            gets_plus_one_plus_two_and_flying,
            "Whenever this creature attacks while saddled, it gets +1/+2 and gains flying until end of turn.",
        ),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BRIGHTFIELD_GLIDER
};
