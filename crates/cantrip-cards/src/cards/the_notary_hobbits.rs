use cantrip_api::*;
use cantrip_mechanics::{copy, lands, mana, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Create two tokens that are copies of them, except the tokens aren't legendary." (CR 707.2, 707.9b, 111.1): both at once, copies
/// of the copiable values of The Notary Hobbits as they are, or as they last were if they have left the battlefield (CR 707.2,
/// 608.2h), with the one exception stated, so they are Halflings with these two abilities and are not legendary (CR 205.4).
fn make_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    copy::create_token_copies_of_it(g, item, 2, |_, values| values.nonlegendary())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            make_copies,
            "When The Notary Hobbits enter, if they're not a token, create two tokens that are copies of them, except the tokens aren't legendary.",
        )
        .condition(copy::not_a_token),
        mana_ability(TAP, add_by(mana::add_for_each_subtype_you_control::<{ Mana::C as u8 }, { lands::sub(Subtype::Halfling) }>), "{T}: Add {C} for each Halfling you control."),
    ],
    ..oracle::THE_NOTARY_HOBBITS
};
