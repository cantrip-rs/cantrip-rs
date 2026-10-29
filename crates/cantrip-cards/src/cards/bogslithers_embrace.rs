use cantrip_api::*;
use cantrip_mechanics::{blight, exile_target, targets};
use cantrip_oracle as oracle;

/// "Exile target creature" (CR 701.13a): nothing if it is no longer a legal target as the spell resolves (CR 608.2b).
fn exile_the_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], exile_the_creature),
    // "As an additional cost to cast this spell, blight 1 or pay {3}" (CR 601.2b, 701.68a).
    additional_choice: &[
        ExtraCost { text: "Blight 1", cost: blight::cost(1) },
        ExtraCost { text: "Pay {3}", cost: Cost::mana(mana("{3}")) },
    ],
    ..oracle::BOGSLITHERS_EMBRACE
};
