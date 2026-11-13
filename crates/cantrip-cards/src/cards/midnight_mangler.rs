use cantrip_api::*;
use cantrip_mechanics::{crew::crew, statics::creature_too};
use cantrip_oracle as oracle;

/// "During turns other than yours" (CR 604.1): this Vehicle, while the player whose turn it is is not its controller.
fn itself_during_turns_other_than_yours(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().turn.active != q.state().obj(source).controller
}

pub static CARD: CardDef = CardDef {
    abilities: &[crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)")],
    statics: &[static_ability(
        Layer::Type,
        itself_during_turns_other_than_yours,
        creature_too,
        "During turns other than yours, this Vehicle is an artifact creature.",
    )],
    ..oracle::MIDNIGHT_MANGLER
};
