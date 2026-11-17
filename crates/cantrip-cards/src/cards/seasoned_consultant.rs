use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you attack with three or more creatures, this creature gets +2/+0 until end of turn" (CR 508.1, 603.2, 611.2a): once
/// for the whole attack however many creatures attack; the Consultant need not be one of them, and it gets the pump as the
/// ability resolves whatever happens to the attackers in response.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, triggers::you_attack_with_at_least::<3>),
        effects::this_gets::<2, 0>,
        "Whenever you attack with three or more creatures, this creature gets +2/+0 until end of turn.",
    )],
    ..oracle::SEASONED_CONSULTANT
};
