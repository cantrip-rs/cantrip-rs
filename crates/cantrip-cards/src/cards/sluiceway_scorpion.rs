use cantrip_api::*;
use cantrip_mechanics::scavenge;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[scavenge::ability(
        mana("{1}{B}{G}"),
        "Scavenge {1}{B}{G} ({1}{B}{G}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Scavenge only as a sorcery.)",
    )],
    ..oracle::SLUICEWAY_SCORPION
};
