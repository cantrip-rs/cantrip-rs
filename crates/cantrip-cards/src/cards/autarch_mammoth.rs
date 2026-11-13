use cantrip_api::*;
use cantrip_mechanics::{saddle, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters and whenever it attacks while saddled" is one ability with two trigger conditions (CR 603.2): its own
/// entering (`this_enters`), or its own attack while saddled (`this_attacks_while_saddled`). One event can be only one of them.
fn enters_or_attacks_while_saddled(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    triggers::this_enters(q, source, event).or_else(|| triggers::this_attacks_while_saddled(q, source, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::ATTACKS.0), enters_or_attacks_while_saddled),
            tokens::create_a_3_3_elephant,
            "When this creature enters and whenever it attacks while saddled, create a 3/3 green Elephant creature token.",
        ),
        saddle::ability(5, "Saddle 5 (Tap any number of other creatures you control with total power 5 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::AUTARCH_MAMMOTH
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEPHANT_3_3];
