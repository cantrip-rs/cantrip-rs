use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "...if they're not a token" (CR 603.4): an intervening "if", asked as it triggers and again as it resolves, of the permanent as
/// it is, or as it last was if it has left the battlefield (CR 608.2h). A token copy of them enters and triggers nothing.
fn not_a_token(q: &dyn Query, asker: &Asker) -> bool {
    !q.obj(asker.source).token
}

/// "Create two tokens that are copies of them, except the tokens aren't legendary." (CR 707.2, 707.9b, 111.1): both at once, copies
/// of the copiable values of The Notary Hobbits as they are, or as they last were if they have left the battlefield (CR 707.2,
/// 608.2h), with the one exception stated, so they are Halflings with these two abilities and are not legendary (CR 205.4).
fn make_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let values = g.copiable_values(item.object).nonlegendary();
    g.create_token_copies_of(values, item.controller, 2)?;
    Ok(())
}

/// "Add {C} for each Halfling you control" (CR 106.5): counted as the ability is activated, possibly none (it can't be, as
/// the source is one while it taps), with Halfling a word of this card's text (CR 612) and the creatures there as the layers have
/// them. Opponents' Halflings do not count.
fn add_colorless_for_each_halfling(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let halfling = g.words(asker.source).subtype(Subtype::Halfling);
    let halflings = g.permanents_of(asker.controller).into_iter().filter(|&id| g.chars(id).has_subtype(halfling)).count();
    g.add_mana(asker.controller, Mana::C, halflings as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            make_copies,
            "When The Notary Hobbits enter, if they're not a token, create two tokens that are copies of them, except the tokens aren't legendary.",
        )
        .condition(not_a_token),
        mana_ability(TAP, add_by(add_colorless_for_each_halfling), "{T}: Add {C} for each Halfling you control."),
    ],
    ..oracle::THE_NOTARY_HOBBITS
};
