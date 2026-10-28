use cantrip_api::*;
use cantrip_mechanics::statics::{other_creatures_you_control, plus_pt};
use cantrip_mechanics::{recruit, tokens, triggers};
use cantrip_oracle as oracle;

/// "You may cast this spell as though it had flash if you control a Human" (CR 307.5, 702.8): asked of the player who would cast
/// it as the casting begins, and not again (the rulings). A Human is a permanent with the creature type Human, read through the
/// words of this card's text (CR 612); an opponent's does not count, and neither does one that is not on the battlefield.
fn you_control_a_human(q: &dyn Query, asker: &Asker) -> bool {
    let human = q.words(asker.source).subtype(Subtype::Human);
    q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).has_subtype(human))
}

pub static CARD: CardDef = CardDef {
    flash_if: Some(you_control_a_human),
    statics: &[static_ability(Layer::PtModify, other_creatures_you_control, plus_pt::<1, 1>, "Other creatures you control get +1/+1.")],
    abilities: &[triggered(triggers::THIS_ENTERS_OR_ATTACKS, recruit::effect, "Whenever this creature enters or attacks, recruit.")],
    ..oracle::BARDS_COMPANY
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
