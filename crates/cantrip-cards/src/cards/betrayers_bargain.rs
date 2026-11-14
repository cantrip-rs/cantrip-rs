use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::damage_target_exile_if_it_would_die::<5>),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    // As an additional cost to cast this spell, sacrifice a creature or enchantment or pay {2} (CR 601.2b, 118.8).
    additional_choice: &[
        ExtraCost { text: "Sacrifice a creature or enchantment", cost: Cost::sacrifice(1, targets::filters::creature_or_enchantment) },
        ExtraCost { text: "Pay {2}", cost: Cost::mana(mana("{2}")) },
    ],
    ..oracle::BETRAYERS_BARGAIN
};
