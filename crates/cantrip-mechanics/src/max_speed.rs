//! Max speed (CR 702.178): "Max speed — [Ability]" means "As long as your speed is 4, this object has '[Ability].'" A
//! static ability of the permanent that grants it the ability its card lists first in `delayed` (CR 613.1f, layer 6),
//! so the permanent does not have that ability at all below max speed - it can't be activated and it doesn't trigger -
//! and an effect that removes the permanent's abilities removes the grant with them. "Your" is its controller's
//! (CR 109.5), so a permanent whose control changes has the ability by its new controller's speed. Perilous Snare prints
//! it. A card whose granted ability functions from its owner's graveyard (CR 702.178b: the static ability functions from
//! the zone the granted ability does) is built with `max_speed_in_graveyard`; Leonin Surveyor and Lightwheel Enhancements
//! print it.

use cantrip_api::*;

use crate::start_your_engines::has_max_speed;

/// "As long as your speed is 4" (CR 702.178a): the permanent itself, while its controller has max speed. Perilous Snare's granted ability and Burnout Bashtronaut's double strike use it.
pub fn itself_at_max_speed(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && has_max_speed(view.state(), view.state().obj(source).controller)
}

/// "Max speed — [Ability]" (CR 702.178a), for the card's `statics`: the card's delayed ability 0 is the granted one, and
/// `text` is the whole line as printed.
pub const fn max_speed(text: &'static str) -> StaticDef {
    static_ability(Layer::Ability, itself_at_max_speed, crate::statics::grant_delayed_ability, text)
}

/// "Max speed — [Ability]" (CR 702.178a) of a card whose ability functions from its owner's graveyard (CR 702.178b), for the
/// card's `statics`: as long as its owner's speed is 4, the card in their graveyard has what `grant` gives it (`grant_delayed_ability`
/// for an activated ability the card lists first in `delayed`, with `.from(ZoneKind::Graveyard)` on it; a keyword for
/// "You may cast this card from your graveyard"). A card in a graveyard has no controller, so its owner stands in (CR 108.4a):
/// "your speed" is theirs. The static ability applies to cards outside the battlefield (`StaticDef::everywhere`, CR 611.3a)
/// and is the card's own alone.
pub const fn max_speed_in_graveyard(grant: StaticApply, text: &'static str) -> StaticDef {
    static_ability(Layer::Ability, itself_at_max_speed, grant, text).from(ZoneKind::Graveyard).everywhere()
}
