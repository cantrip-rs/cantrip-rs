use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this Vehicle enters, draw two cards" (CR 121.1).
fn draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, draw_two, "When this Vehicle enters, draw two cards."),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::HULLDRIFTER
};
