use cantrip_api::*;
use cantrip_mechanics::{effects, history, lands, statics};
use cantrip_oracle as oracle;

const CELEBRATION: &str = "Celebration — As long as two or more nonland permanents entered the battlefield under your control this turn, Goddric is a Dragon with base power and toughness 4/4, flying, and \"{R}: Dragons you control get +1/+0 until end of turn.\"";

/// "... has flying, and \"{R}: Dragons you control get +1/+0 until end of turn.\"" (CR 613.1f, layer 6): flying and the
/// granted activated ability (the card's `delayed` entry 0) in the one effect.
fn flying_and_dragon_pump(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::have_flying(view, source, chars);
    statics::has_delayed_ability::<0>(view, source, chars);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Type, history::this_while_you_celebrate, statics::creature_type_alone::<{ lands::sub(Subtype::Dragon) }>, CELEBRATION),
        static_ability(Layer::Ability, history::this_while_you_celebrate, flying_and_dragon_pump, CELEBRATION).continues(),
        static_ability(Layer::PtSet, history::this_while_you_celebrate, statics::base_pt::<4, 4>, CELEBRATION).continues(),
    ],
    delayed: &[ability(cost("{R}"), effects::dragons_you_control_get_plus_n_zero::<1>, "{R}: Dragons you control get +1/+0 until end of turn.")],
    ..oracle::GODDRIC_CLOAKED_REVELER
};
