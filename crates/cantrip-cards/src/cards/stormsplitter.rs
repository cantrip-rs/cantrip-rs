use cantrip_api::*;
use cantrip_mechanics::triggers::INSTANT_OR_SORCERY_YOU_CAST;
use cantrip_mechanics::{copy, delayed};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell, create a token that's a copy of this creature. Exile that token at the beginning
/// of the next end step." (CR 603.2, 707.2, 603.7c): the token is a copy of this creature's copiable values as last it existed on the
/// battlefield (ruling 2024-07-26), none of its counters or other effects, and it has this ability too.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        INSTANT_OR_SORCERY_YOU_CAST,
        |g, item| {
            let values = g.copiable_values(item.object);
            copy::create_token_copy_exiled_at_next_end_step(g, item, values, 0)
        },
        "Whenever you cast an instant or sorcery spell, create a token that's a copy of this creature. Exile that token at the beginning of the next end step.",
    )],
    delayed: &[delayed_trigger(delayed::exile_subject, "Exile that token at the beginning of the next end step.")],
    ..oracle::STORMSPLITTER
};
