use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER;
use cantrip_oracle as oracle;

/// "This creature gets +2/+0 and gains vigilance until end of turn" (CR 611.2a): both changes come from this resolution and do
/// nothing once the creature has left the battlefield (CR 400.7).
fn pump_and_vigilance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, item.object, 2, 0);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Vigilance);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
        pump_and_vigilance,
        "Whenever you cast a spell with mana value 4 or greater, this creature gets +2/+0 and gains vigilance until end of turn.",
    )],
    ..oracle::KULRATH_MYSTIC
};
