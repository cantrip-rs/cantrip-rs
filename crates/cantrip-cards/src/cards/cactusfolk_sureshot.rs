use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Other creatures you control with power 4 or greater gain trample and haste until end of turn" (CR 611.2c, 702.19, 702.10):
/// the creatures are those with that power as the ability resolves (CR 208.1); one whose power drops later keeps the keywords,
/// and one that enters or grows later gains nothing.
fn others_with_power_four_gain_trample_and_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let source = item.object;
    effects::creatures_you_control_where_gain(g, item, &[Keyword::Trample, Keyword::Haste], move |g, id| {
        id != source && g.chars(id).power.is_some_and(|power| power >= 4)
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            triggers::BEGINNING_OF_YOUR_COMBAT,
            others_with_power_four_gain_trample_and_haste,
            "At the beginning of combat on your turn, other creatures you control with power 4 or greater gain trample and haste until end of turn.",
        ),
    ],
    ..oracle::CACTUSFOLK_SURESHOT
};
