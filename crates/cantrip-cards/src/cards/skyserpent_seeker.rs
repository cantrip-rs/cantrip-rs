use cantrip_api::*;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

/// "Reveal cards from the top of your library until you reveal two land cards. Put those land cards onto the battlefield tapped
/// and the rest on the bottom of your library in a random order. Put a +1/+1 counter on this creature." (CR 701.20, 122.1a): two
/// sentences, so two instructions (CR 608.2c); the counter goes on even if the library held no land.
fn dig_for_two_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::reveal_until_two_lands_onto_battlefield_tapped(g, item.controller)?;
    g.end_sentence();
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}"),
        dig_for_two_lands,
        "Exhaust \u{2014} {4}: Reveal cards from the top of your library until you reveal two land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order. Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::SKYSERPENT_SEEKER
};
