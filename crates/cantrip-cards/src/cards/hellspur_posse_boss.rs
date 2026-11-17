use cantrip_api::*;
use cantrip_mechanics::{outlaw, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Other outlaws you control have haste" (CR 700.12, 613.1f): every other permanent its controller controls that is an
/// outlaw (CR 700.12), judged on what the earlier layers made it, so a creature that becomes a Rogue in layer 4 is one here.
fn other_outlaws_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::other_permanents_you_control(q, source, affected, so_far) && outlaw::is_outlaw(so_far)
}

/// "Create two 1/1 red Mercenary creature tokens ..." (CR 111.1, 603.6a): both at once.
fn create_two_mercenaries(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::MERCENARY, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_two_mercenaries,
        "When this creature enters, create two 1/1 red Mercenary creature tokens with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
    )],
    statics: &[static_ability(
        Layer::Ability,
        other_outlaws_you_control,
        statics::have_haste,
        "Other outlaws you control have haste.",
    )],
    ..oracle::HELLSPUR_POSSE_BOSS
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
