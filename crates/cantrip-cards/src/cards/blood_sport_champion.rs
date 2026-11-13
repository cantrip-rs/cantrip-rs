use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::kicker::was_kicked;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Other creatures you control gain deathtouch until end of turn" (CR 611.2, 611.2c, 702.2): each other creature its controller
/// controls as the trigger resolves, and no creature that enters later - the champion itself already has deathtouch, so leaving it
/// out changes nothing it shows.
fn others_gain_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::other_creatures_you_control_gain(g, item, &[Keyword::Deathtouch]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{2}")),
    abilities: &[triggered(
        THIS_ENTERS,
        others_gain_deathtouch,
        "When this creature enters, if it was kicked, other creatures you control gain deathtouch until end of turn.",
    )
    .condition(was_kicked)],
    ..oracle::BLOOD_SPORT_CHAMPION
};
