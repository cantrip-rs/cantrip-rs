use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// I, II, III, IV: "Create three 2/2 white Knight creature tokens." (CR 111.1, 714.2b)
fn knights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KNIGHT_2_2_WHITE, 3)?;
    Ok(())
}

/// V — Ultimate End: "Other creatures you control get +2/+2 until end of turn. Put an indestructible counter on each of
/// them." (CR 611.2c, 122.1b): the creatures its controller controls as it resolves but the Saga; two sentences, so two
/// instructions (CR 608.2c), and "each of them" is those same creatures, the counters put on all at once (CR 101.4).
fn ultimate_end(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others = effects::pump_other_creatures_you_control(g, item, 2, 2);
    g.end_sentence();
    effects::counters_on_each(g, &others, CounterKind::Indestructible, 1);
    Ok(())
}

const KNIGHTS: &str = "I, II, III, IV \u{2014} Create three 2/2 white Knight creature tokens.";

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, knights, KNIGHTS),
        chapter(2, knights, KNIGHTS),
        chapter(3, knights, KNIGHTS),
        chapter(4, knights, KNIGHTS),
        chapter(5, ultimate_end, "V \u{2014} Ultimate End \u{2014} Other creatures you control get +2/+2 until end of turn. Put an indestructible counter on each of them."),
    ],
    ..oracle::SUMMON_KNIGHTS_OF_ROUND
};

pub static EXTRA: &[&CardDef] = &[&tokens::KNIGHT_2_2_WHITE];
