use cantrip_api::*;
use cantrip_mechanics::{behold, subtype_cards_in_graveyard};
use cantrip_oracle as oracle;

/// "Activate only if there is an Elf card in your graveyard" (CR 602.5b): checked as the ability is activated, the Elf read
/// through the words of this card's text (CR 612).
fn elf_card_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    subtype_cards_in_graveyard(q, asker.controller, q.words(asker.source).subtype(Subtype::Elf)) > 0
}

/// "As an additional cost to cast this spell, behold an Elf or pay {2}" (CR 601.2b, 701.4a): one of the two is chosen.
pub static CARD: CardDef = CardDef {
    additional_choice: &[
        ExtraCost { text: "Behold an Elf", cost: behold::cost(behold::elf, "Elf") },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    abilities: &[mana_ability(TAP, add_n(Mana::G, 2), "{T}: Add {G}{G}. Activate only if there is an Elf card in your graveyard.")
        .condition(elf_card_in_your_graveyard)],
    ..oracle::LYS_ALANA_DIGNITARY
};
