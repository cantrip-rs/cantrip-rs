use cantrip_api::*;
use cantrip_mechanics::{history, statics::plus_pt};
use cantrip_oracle as oracle;

/// "... as long as you've cast two or more spells this turn" (CR 611.3a, 601.2i) as a static ability's `StaticDef::affects`: only the
/// source itself, and only while its controller has cast two or more spells this turn, itself included if it was cast this turn (it
/// is one of them once cast). The layers ask again whenever characteristics are computed, so it turns on with the second cast and
/// off as the turn ends. Spells that were countered or are still on the stack count (the history holds every cast).
fn itself_while_you_cast_two_or_more_spells(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && history::cast_two_or_more_spells_this_turn(q.history(), q.state().obj(source).controller)
}

/// Brightspear Zealot: vigilance is a core keyword.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_cast_two_or_more_spells,
        plus_pt::<2, 0>,
        "This creature gets +2/+0 as long as you've cast two or more spells this turn.",
    )],
    ..oracle::BRIGHTSPEAR_ZEALOT
};
