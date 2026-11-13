//! Land's Edge reads a discarded card's defined characteristics (CR 701.9c).
#[path = "support/discard_library.rs"]
mod discard_library;
use discard_library::play;

fn script(leng: &str, life: u32) -> String {
    format!(r#"
[Bot1 Deck]
{leng}
Black Lotus
Land's Edge
Mountain
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Land's Edge.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Land's Edge.Activate(Bot2)
Bot1.Choose(Mountain)
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = {life}
"#)
}

#[test]
fn land_discarded_unrevealed_has_no_defined_type() {
    play("lands-edge-unrevealed", &script("Probe Leng", 20));
}

#[test]
fn land_discarded_revealed_has_its_type() {
    play("lands-edge-revealed", &script("Probe Revealing Leng", 18));
}
