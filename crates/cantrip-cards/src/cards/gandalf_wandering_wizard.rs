use cantrip_api::*;
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

/// "Gandalf's owner shuffles him into their library and draws three cards." (CR 701.24, 121.1): the owner, not the
/// controller, does both; the library is shuffled even if Gandalf has left the battlefield (CR 701.24c).
fn shuffle_in_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    g.put_into_library(&[item.object], LibraryPlace::ShuffledIn);
    g.draw(owner, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)"),
        ability(cost("{6}"), shuffle_in_and_draw, "{6}: Gandalf's owner shuffles him into their library and draws three cards."),
    ],
    ..oracle::GANDALF_WANDERING_WIZARD
};
