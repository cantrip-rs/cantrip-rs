use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL;
use cantrip_oracle as oracle;

/// Double strike is a core keyword. "Whenever you cast a spell that targets a creature you control, put a +1/+1 counter on
/// Mockingbird" (CR 603.2, 122.1): one for each such spell, including one that targets Mockingbird herself.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL,
        plus_one_counter_on_this,
        "Whenever you cast a spell that targets a creature you control, put a +1/+1 counter on Mockingbird.",
    )],
    ..oracle::MOCKINGBIRD_ACE_AGENT
};
