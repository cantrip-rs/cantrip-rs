use cantrip_api::*;
use cantrip_mechanics::{statics, warp};
use cantrip_oracle as oracle;

/// "Artifact cards and red creature cards in your hand have warp {2}{R}" (CR 702.185a, 113.6, 604): the cards in its
/// controller's hand, cast by that controller, that are artifact cards or red creature cards there. The card is read as
/// the face cast (`q` reads the spell as it, CR 709.3a).
fn warp_cost(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> Option<Cost> {
    if caster != asker.controller || q.obj(card).owner != asker.controller {
        return None;
    }
    let chars = q.chars(card);
    let gets = chars.is(CardType::Artifact) || (chars.is(CardType::Creature) && chars.colors.has(q.words(asker.source).color(Color::Red)));
    gets.then(|| Cost::mana(mana("{2}{R}")))
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::other_creatures_you_control, statics::have_haste, "Other creatures you control have haste.")],
    granted_ways: &[warp::granted(warp_cost)],
    delayed: &[warp::EXILE],
    ..oracle::TANNUK_STEADFAST_SECOND
};
