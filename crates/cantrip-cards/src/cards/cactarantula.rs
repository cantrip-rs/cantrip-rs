use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This spell costs {1} less to cast if you control a Desert" (CR 601.2f, 118.7a). Reach is a core keyword.
    own_cost_changes: &[cost_reduction::less_if_you_control_subtype::<1, { lands::sub(Subtype::Desert) }>],
    abilities: &[triggered(
        triggers::THIS_BECOMES_THE_TARGET_OF_AN_OPPONENTS_SPELL_OR_ABILITY,
        // "You may draw a card" (CR 121.1, 603.3): the trigger resolves before the spell or ability that caused it, once however
        // many times the creature was targeted at once.
        effects::may_draw_a_card,
        "Whenever this creature becomes the target of a spell or ability an opponent controls, you may draw a card.",
    )],
    ..oracle::CACTARANTULA
};
