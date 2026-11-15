//! Sets for the rules a resolving spell or ability makes (`LastingRule`, CR 611.2c).

use cantrip_api::*;

/// The one object the rule's words named as it began (`about`, `Game::add_lasting_rule`): "that creature", "that token". A
/// permanent that leaves and returns is a new object the rule never names (CR 400.7). Whippoorwill's "damage that would be
/// dealt to that creature this turn can't be prevented or dealt instead to another permanent or player", Howlsquad Heavy's
/// "that token attacks this combat if able", and Nettling Imp's, Norritt's and Arcum's Whistle's "that creature attacks this
/// turn if able" (`combat::MUST_ATTACK_THIS_TURN`).
pub fn the_object_named(_: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    about == Target::Object(id)
}
