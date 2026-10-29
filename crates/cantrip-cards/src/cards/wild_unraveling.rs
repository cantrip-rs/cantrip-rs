use cantrip_api::*;
use cantrip_mechanics::{blight, counter_target, targets};
use cantrip_oracle as oracle;

fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    // "As an additional cost to cast this spell, blight 2 or pay {1}" (CR 601.2b, 701.68a).
    additional_choice: &[
        ExtraCost { text: "Blight 2", cost: blight::cost(2) },
        ExtraCost { text: "Pay {1}", cost: Cost::mana(mana("{1}")) },
    ],
    ..oracle::WILD_UNRAVELING
};
