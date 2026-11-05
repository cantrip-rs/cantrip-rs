use cantrip_api::*;
use cantrip_mechanics::{counters_divided, effects, endure, targets};
use cantrip_oracle as oracle;

/// "I — Create three 1/1 white Spirit creature tokens" (CR 111.1, 714.2): the white Spirit endure makes, sized 1/1 as it is
/// made, as Great Arashin City makes it; the three are created together, by one instruction.
fn create_three_spirits(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.create_token_copies_of(Copiable::of(g.card_id(&endure::SPIRIT)).sized(1, 1), item.controller, 3)?;
    Ok(())
}

/// "II — Distribute three +1/+1 counters among one, two, or three target creatures you control." The targets and their shares
/// are fixed as the chapter ability goes on the stack, each target getting at least one (CR 601.2d, 603.3d); a target illegal as
/// it resolves takes its share with it (CR 608.2b).
fn distribute(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_divided(g, item, CounterKind::PlusOne);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, create_three_spirits, "I \u{2014} Create three 1/1 white Spirit creature tokens."),
        chapter(2, distribute, "II \u{2014} Distribute three +1/+1 counters among one, two, or three target creatures you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL.between(1, 3)])
            .divide(Divide { total: 3, reads: None }),
        // The creatures its controller controls as chapter III resolves, and no creature that enters later (CR 611.2c).
        chapter(3, |g, item| {
            effects::creatures_you_control_gain(g, item, &[Keyword::Trample, Keyword::Lifelink]);
            Ok(())
        }, "III \u{2014} Creatures you control gain trample and lifelink until end of turn."),
    ],
    ..oracle::REVIVAL_OF_THE_ANCESTORS
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
