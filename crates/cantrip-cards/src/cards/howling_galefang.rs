use cantrip_api::*;
use cantrip_mechanics::{library, statics::have_haste};
use cantrip_oracle as oracle;

/// "This creature has haste as long as you own a card in exile that has an Adventure" (CR 604.1, 611.3a, 715.3d) as a static
/// ability's `StaticDef::affects`: only the source itself, and only while its controller's exile zone (a card is exiled to its
/// owner's zone, so these are the cards its controller owns) holds a face-up card with an Adventure, whether it is on an adventure there or
/// was exiled some other way. An opponent's Adventure card in exile does not count.
fn itself_while_you_own_an_adventure_card_in_exile(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    source == affected
        && state.zone(Zone::exile(state.obj(source).controller)).iter().any(|&id| {
        let obj = state.obj(id);
        library::object_has_an_adventure(obj, q.cards().get(obj.card))
    })
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_own_an_adventure_card_in_exile,
        have_haste,
        "This creature has haste as long as you own a card in exile that has an Adventure.",
    )],
    ..oracle::HOWLING_GALEFANG
};
