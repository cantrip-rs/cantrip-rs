use cantrip_api::*;
use cantrip_mechanics::triggers::ward_collect_evidence;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ward_collect_evidence::<4>(
        "Ward\u{2014}Collect evidence 4. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player exiles cards with total mana value 4 or greater from their graveyard.)",
    )],
    ..oracle::AXEBANE_FEROX
};
