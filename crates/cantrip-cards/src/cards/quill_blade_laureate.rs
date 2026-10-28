use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, pump, targets};
use cantrip_oracle as oracle;

/// "Twofold Intent - Target creature gets +1/+0 and gains double strike until end of turn."
fn twofold_intent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::DoubleStrike);
    });
    Ok(())
}

/// Quill-Blade Laureate's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static TWOFOLD_INTENT: CardDef = CardDef {
    name: "Twofold Intent",
    cost: mana("{1}{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE], twofold_intent),
    text: "Target creature gets +1/+0 and gains double strike until end of turn.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&TWOFOLD_INTENT),
    ..oracle::QUILL_BLADE_LAUREATE
};

pub static EXTRA: &[&CardDef] = &[&TWOFOLD_INTENT];
