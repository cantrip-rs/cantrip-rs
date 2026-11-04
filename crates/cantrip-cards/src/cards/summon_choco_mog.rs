use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// I, II, III, IV — Stampede!: "Other creatures you control get +1/+0 until end of turn." (CR 611.2c, 714.2b): the creatures
/// its controller controls as the chapter resolves but the Saga, and none that enters later.
fn stampede(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_other_creatures_you_control(g, item, 1, 0);
    Ok(())
}

const STAMPEDE: &str = "I, II, III, IV \u{2014} Stampede! \u{2014} Other creatures you control get +1/+0 until end of turn.";

pub static CARD: CardDef = CardDef {
    abilities: &[chapter(1, stampede, STAMPEDE), chapter(2, stampede, STAMPEDE), chapter(3, stampede, STAMPEDE), chapter(4, stampede, STAMPEDE)],
    ..oracle::SUMMON_CHOCO_MOG
};
