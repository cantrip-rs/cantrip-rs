use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::ELEPHANT_3_3];

/// "Put a +1/+1 counter on this creature. Create a 3/3 green Elephant creature token." (CR 122.1, 111.1): two sentences, so
/// the token is made even when this creature has left the battlefield and no counter could go on it (CR 400.7).
fn counter_then_elephant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    tokens::create_a_3_3_elephant(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{G}"),
        counter_then_elephant,
        "Exhaust \u{2014} {3}{G}: Put a +1/+1 counter on this creature. Create a 3/3 green Elephant creature token. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::STAMPEDING_SCURRYFOOT
};
