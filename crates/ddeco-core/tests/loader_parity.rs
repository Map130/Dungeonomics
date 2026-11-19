//! R2: the TOML loader reads the same world as Python (`tests/test_loader.py`).

use ddeco_core::loader::world_from_toml;
use ddeco_core::world::NodeKind;

fn example_world() -> ddeco_core::world::World {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/example_region.toml"
    ))
    .expect("example_region.toml");
    world_from_toml(&text).expect("parse")
}

#[test]
fn loads_nodes_and_edges() {
    let w = example_world();
    assert_eq!(w.seed, 42);
    assert_eq!(w.nodes.len(), 10); // 3 cities + 6 villages + 1 trading post
    assert_eq!(w.edges.len(), 22); // 11 roads → 22 arcs
    assert_eq!(w.nodes["eastport"].kind, NodeKind::Camp);
    assert_eq!(w.nodes["eastport"].hub, None);
    assert_eq!(w.nodes["redwater"].kind, NodeKind::City);
    assert_eq!(w.nodes["millbrook"].kind, NodeKind::Village);
    assert_eq!(w.nodes["millbrook"].hub.as_deref(), Some("redwater"));
    assert!(w.edges.contains_key("redwater__millbrook"));
    assert!(w.edges.contains_key("millbrook__redwater"));
}

#[test]
fn good_overrides_and_class_defaults() {
    let w = example_world();
    let fish = w.catalog.get("fish");
    assert_eq!(fish.spoilage, 0.03); // override from TOML
    assert_eq!(fish.alpha, 0.5); // necessity class default (§7)
    assert!(fish.in_food_basket);
    let food: Vec<&String> = w.catalog.food_ids().collect();
    assert_eq!(food, vec!["fish", "grain", "meat", "salt"]); // sorted by the BTreeMap
    assert_eq!(w.catalog.goods.len(), 11);
}

#[test]
fn base_prices_from_phb() {
    let w = example_world();
    assert_eq!(w.catalog.get("grain").base_price, 0.01);
    assert_eq!(w.catalog.get("wine").base_price, 10.0);
    assert_eq!(w.catalog.get("weapons").base_price, 15.0); // SRD longsword
}

#[test]
fn edge_delay_matches_python() {
    let w = example_world();
    // redwater__stonehaven: travel_time 3, difficulty 1.2 → ceil(3.6) = 4
    assert_eq!(w.edges["redwater__stonehaven"].delay_days(), 4);
    assert_eq!(w.edges["redwater__millbrook"].delay_days(), 1);
}
