use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::you_cast_a_spell_with_mana_value_at_least;
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Scan the Clouds.
pub static SCAN_THE_CLOUDS: CardDef = CardDef { spell: spell(&[], effects::draw_then_discard::<2, 2>), ..oracle::SCAN_THE_CLOUDS };

/// Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_with_mana_value_at_least::<5>),
        effects::plus_one_counter_on_this,
        "Whenever you cast a spell with mana value 5 or greater, put a +1/+1 counter on this creature.",
    )],
    adventure: Some(&SCAN_THE_CLOUDS),
    ..oracle::TEMPEST_HART
};

pub static EXTRA: &[&CardDef] = &[&SCAN_THE_CLOUDS];
