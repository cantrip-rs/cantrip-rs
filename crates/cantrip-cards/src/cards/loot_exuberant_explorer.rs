use cantrip_api::*;
use cantrip_mechanics::{library, lands_you_control};
use cantrip_oracle as oracle;

/// "Look at the top six cards of your library. You may reveal a creature card with mana value less than or equal to the
/// number of lands you control from among them and put it onto the battlefield. Put the rest on the bottom in a random order"
/// (CR 701.20, 401.4, 608.2c): the lands are counted as the card is chosen, as the layers give them now (CR 613.1d), and the
/// mana value is the card's in the library (an {X} in it is 0, CR 202.3e). The card revealed is the one that enters, so it is
/// shown by arriving; a smaller library shows what it has (CR 609.3).
fn look_for_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands = lands_you_control(g, item.controller) as u32;
    let creature = move |q: &dyn Query, card: ObjectId| q.chars(card).is(CardType::Creature) && q.mana_value(card) <= lands;
    library::look_at_top_onto_battlefield(g, item.controller, item.object, 6, 1, "creature", creature, None, false, library::Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{G}{G}, {T}"),
        look_for_a_creature,
        "{4}{G}{G}, {T}: Look at the top six cards of your library. You may reveal a creature card with mana value less than or equal to the number of lands you control from among them and put it onto the battlefield. Put the rest on the bottom in a random order.",
    )],
    // "You may play an additional land on each of your turns" (CR 305.2).
    play_rules: &[PlayRule::ExtraLandPlays(1)],
    ..oracle::LOOT_EXUBERANT_EXPLORER
};
