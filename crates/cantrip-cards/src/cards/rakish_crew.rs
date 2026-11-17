use cantrip_api::*;
use cantrip_mechanics::{effects, outlaw, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever an outlaw you control dies" (CR 700.4, 700.12, 603.10a): a creature you controlled as it last was on the
/// battlefield, and an outlaw by the creature types it last had there (the rulings). Each is its own event, so outlaws that die
/// together, this enchantment leaving with them, trigger it once each.
fn outlaw_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::creature_you_control_dies(q, me, event)?;
    let Event::ZoneChange { old, .. } = event else { return None };
    outlaw::is_outlaw(&q.chars(*old)).then_some(fired)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_a_mercenary,
            "When this enchantment enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, outlaw_you_control_dies),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever an outlaw you control dies, each opponent loses 1 life and you gain 1 life.",
        ),
    ],
    ..oracle::RAKISH_CREW
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
