use cantrip_api::*;
use cantrip_mechanics::effects::of_subtype_you_control_gain_until_end_of_turn;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Create three 1/1 red Gremlin creature tokens. Gremlins you control gain menace, lifelink, and haste until end of turn."
/// (CR 111.1, 611.2c): the Gremlins this player controls as the second sentence is followed, the new tokens among them, and no
/// Gremlin that enters later.
fn three_gremlins_and_gremlins_gain_menace_lifelink_and_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GREMLIN, 3)?;
    g.end_sentence();
    of_subtype_you_control_gain_until_end_of_turn::<{ lands::sub(Subtype::Gremlin) }, { Keywords::of(&[Keyword::Menace, Keyword::Lifelink, Keyword::Haste]).0 }>(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], three_gremlins_and_gremlins_gain_menace_lifelink_and_haste),
    ..oracle::MIDNIGHT_MAYHEM
};

pub static EXTRA: &[&CardDef] = &[&tokens::GREMLIN];
