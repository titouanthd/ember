//! `TileSet` and the `TileMap` node (ADR-053): cells and rows, cell <->
//! world, the tile-set file, drawing, and `.escene` round trips.

use ember_core::math::Rect;
use ember_core::{ClassDB, Color};
use ember_scene::prelude::*;
use ember_scene::tiles::EMPTY;
use ember_servers::{MockRenderer, RecordedCall, TextureId};
use glam::Vec2;
use std::sync::Arc;

const INK: Color = Color::new(0.1, 0.1, 0.1, 1.0);
const PAPER: Color = Color::new(0.9, 0.9, 0.8, 1.0);

fn solid_set() -> Arc<TileSet> {
    Arc::new(
        TileSet::new()
            .with_tile(Tile::solid('#', "wall", INK))
            .with_tile(Tile::solid('.', "floor", PAPER)),
    )
}

fn board() -> TileMap {
    TileMap::from_rows(&["####", "#..#", "####"], Vec2::splat(10.0))
}

// --- Cells and rows ---

#[test]
fn rows_read_and_write_back() {
    let m = board();
    assert_eq!((m.width(), m.height()), (4, 3));
    assert_eq!(m.cell(0, 0), Some('#'));
    assert_eq!(m.cell(1, 1), Some('.'));
    assert_eq!(m.cell(4, 0), None, "out of bounds");
    assert_eq!(m.rows(), ["####", "#..#", "####"]);
}

#[test]
fn short_rows_are_padded_and_spaces_are_empty() {
    let m = TileMap::from_rows(&["#", "# #", ""], Vec2::ONE);
    assert_eq!((m.width(), m.height()), (3, 3));
    assert_eq!(m.rows(), ["#  ", "# #", "   "]);
    assert_eq!(m.cell(1, 1), None);
    assert_eq!(
        m.used_cells().collect::<Vec<_>>(),
        [(0, 0, '#'), (0, 1, '#'), (2, 1, '#')]
    );
    let none = TileMap::from_rows(&[], Vec2::ONE);
    assert_eq!((none.width(), none.height()), (0, 0));
    assert!(none.rows().is_empty());
    let blank = TileMap::from_rows(&["", ""], Vec2::ONE);
    assert_eq!((blank.width(), blank.height()), (0, 0));
}

#[test]
fn set_cell_fill_and_resize() {
    let mut m = board();
    assert!(m.set_cell(1, 1, Some('L')));
    assert!(m.set_cell(2, 1, None));
    assert!(!m.set_cell(9, 9, Some('#')), "out of bounds");
    assert!(!m.set_cell(0, 0, Some('\n')), "control char");
    assert_eq!(m.rows()[1], "#L #");

    m.resize(2, 4);
    assert_eq!(m.rows(), ["##", "#L", "##", "  "]);
    m.resize(3, 1);
    assert_eq!(m.rows(), ["## "]);
    m.fill(Some('.'));
    assert_eq!(m.rows(), ["..."]);
    m.fill(None);
    assert_eq!(m.used_cells().count(), 0);
    assert_eq!(
        TileMap::new(2, 1, Vec2::ONE).rows(),
        [String::from_iter([EMPTY; 2])]
    );
}

// --- Cell <-> world ---

#[test]
fn cell_and_world_coordinates() {
    let m = board().at(Vec2::new(100.0, 50.0));
    assert_eq!(m.cell_to_world(0, 0), Vec2::new(100.0, 50.0));
    assert_eq!(m.cell_to_world(2, 1), Vec2::new(120.0, 60.0));
    assert_eq!(m.cell_center(2, 1), Vec2::new(125.0, 65.0));
    assert_eq!(m.cell_rect(3, 2), Rect::new(130.0, 70.0, 10.0, 10.0));
    assert_eq!(m.bounds(), Rect::new(100.0, 50.0, 40.0, 30.0));

    assert_eq!(m.world_to_cell(Vec2::new(125.0, 65.0)), Some((2, 1)));
    assert_eq!(
        m.world_to_cell(Vec2::new(120.0, 60.0)),
        Some((2, 1)),
        "edge: right/below"
    );
    assert_eq!(m.world_to_cell(Vec2::new(139.9, 79.9)), Some((3, 2)));
    assert_eq!(
        m.world_to_cell(Vec2::new(140.0, 60.0)),
        None,
        "right of the map"
    );
    assert_eq!(
        m.world_to_cell(Vec2::new(99.0, 60.0)),
        None,
        "left of the map"
    );
    // Every cell's centre maps back to it.
    for row in 0..m.height() {
        for col in 0..m.width() {
            assert_eq!(m.world_to_cell(m.cell_center(col, row)), Some((col, row)));
        }
    }
    let mut flat = board();
    flat.cell_size = Vec2::new(0.0, 10.0);
    assert_eq!(flat.world_to_cell(Vec2::ONE), None);
}

// --- TileSet file ---

const SET: &str = r#"(
    texture: "tiles.png",
    tile_size: (16.0, 16.0),
    spacing: 2.0,
    margin: 1.0,
    tiles: [
        (symbol: '#', name: "wall", atlas: Some((0, 0))),
        (symbol: '.', name: "floor", atlas: Some((1, 0)), color: (1.0, 1.0, 1.0, 0.5)),
        (symbol: 'x', name: "hole", color: (0.0, 0.0, 0.0, 1.0)),
    ],
)"#;

#[test]
fn tile_set_file_round_trips() {
    let set = TileSet::from_ron(SET).unwrap();
    assert_eq!(set.texture, "tiles.png");
    assert_eq!(set.tile_size, Vec2::splat(16.0));
    assert_eq!(set.tile('#').unwrap().name, "wall");
    assert_eq!(set.tile('#').unwrap().color, Color::WHITE, "default tint");
    assert_eq!(set.tile('x').unwrap().atlas, None);
    assert!(set.tile('?').is_none());
    assert_eq!(set.texture_id, None);
    assert_eq!(TileSet::from_ron(&set.to_ron()).unwrap(), set);

    let minimal = TileSet::from_ron(r#"(tiles: [(symbol: '#')])"#).unwrap();
    assert!(minimal.texture.is_empty());
    assert!(!minimal.to_ron().contains("texture:"));
}

#[test]
fn atlas_regions_follow_margin_and_spacing() {
    let set = TileSet::from_ron(SET).unwrap();
    assert_eq!(set.region(0, 0), Rect::new(1.0, 1.0, 16.0, 16.0));
    assert_eq!(set.region(2, 1), Rect::new(37.0, 19.0, 16.0, 16.0));
}

#[test]
fn bad_tile_sets_are_rejected() {
    for text in [
        r#"(tiles: [(symbol: '#'), (symbol: '#')])"#,
        r#"(tiles: [(symbol: ' ')])"#,
        r#"(tiles: [(symbol: '\n')])"#,
        r#"(tiles: [(symbol: "ab")])"#,
    ] {
        assert!(
            matches!(TileSet::from_ron(text), Err(SceneError::Parse(_))),
            "{text}"
        );
    }
}

#[test]
fn tile_set_loads_through_the_asset_server() {
    let dir = std::env::temp_dir().join(format!("ember-tiles-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("board.etileset"),
        r#"(tiles: [(symbol: '#', name: "wall")])"#,
    )
    .unwrap();
    let assets = ember_assets::AssetServer::new(&dir).unwrap();
    let handle = assets.load::<TileSet>("board.etileset").unwrap();
    let mut set = handle.get().clone();
    assert_eq!(set.tile('#').unwrap().name, "wall");
    // No texture: nothing to load.
    let mut r = MockRenderer::new();
    set.load_texture(&assets, &mut r).unwrap();
    assert_eq!(set.texture_id, None);
}

// --- Drawing ---

fn draw(map: TileMap) -> Vec<RecordedCall> {
    let tree = NodeTree::with_root(map);
    let mut r = MockRenderer::new();
    tree.draw(&mut DrawContext::new(&mut r));
    r.take_calls()
        .into_iter()
        .filter(|c| {
            matches!(
                c,
                RecordedCall::DrawRect { .. } | RecordedCall::DrawSprite { .. }
            )
        })
        .collect()
}

#[test]
fn without_a_texture_tiles_draw_as_rects_row_by_row() {
    let mut map = TileMap::from_rows(&["#.", " ?"], Vec2::splat(8.0)).with_tile_set(solid_set());
    map.z = 3.0;
    let calls = draw(map);
    assert_eq!(
        calls,
        [
            RecordedCall::DrawRect {
                rect: Rect::new(0.0, 0.0, 8.0, 8.0),
                color: INK,
                z: 3.0
            },
            RecordedCall::DrawRect {
                rect: Rect::new(8.0, 0.0, 8.0, 8.0),
                color: PAPER,
                z: 3.0
            },
        ],
        "empty and unknown cells draw nothing"
    );
}

#[test]
fn with_a_texture_tiles_draw_their_atlas_cell() {
    let tex = TextureId::new(5);
    let set = Arc::new(TileSet::from_ron(SET).unwrap().with_texture_id(tex));
    let map = TileMap::from_rows(&[".x"], Vec2::splat(32.0))
        .at(Vec2::new(0.0, 100.0))
        .with_tile_set(set);
    assert_eq!(
        draw(map),
        [
            RecordedCall::DrawSprite {
                texture: tex,
                dst: Rect::new(0.0, 100.0, 32.0, 32.0),
                src: Some(Rect::new(19.0, 1.0, 16.0, 16.0)),
                tint: Color::new(1.0, 1.0, 1.0, 0.5),
                z: 0.0
            },
            // No atlas cell: a rect even with a texture.
            RecordedCall::DrawRect {
                rect: Rect::new(32.0, 100.0, 32.0, 32.0),
                color: Color::new(0.0, 0.0, 0.0, 1.0),
                z: 0.0
            },
        ]
    );
}

#[test]
fn no_set_or_hidden_draws_nothing() {
    assert!(draw(board()).is_empty());
    let mut hidden = board().with_tile_set(solid_set());
    hidden.visible = false;
    assert!(draw(hidden).is_empty());
}

#[test]
fn tile_at_reads_through_the_set() {
    let m = board().with_tile_set(solid_set());
    assert_eq!(m.tile_at(0, 0).unwrap().name, "wall");
    assert_eq!(m.tile_at(1, 1).unwrap().name, "floor");
    assert!(board().tile_at(0, 0).is_none(), "no set");
}

// --- .escene ---

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node::<TileMap>();
    db
}

const LEVEL: &str = "(
    format: 1,
    uid: \"uid://level\",
    nodes: [
        (name: \"Board\", class: \"TileMap\", z: -1.0, properties: {
            \"cell_size\": Vec2(48.0, 48.0),
            \"position\": Vec2(16.0, 32.0),
            \"rows\": Array([String(\"#####\"), String(\"#.L.#\"), String(\"#####\")]),
            \"tile_set\": String(\"tiles/courtyard.etileset\"),
        }),
    ],
)";

#[test]
fn a_tile_map_loads_from_an_escene() {
    let scene = PackedScene::from_ron(LEVEL).unwrap();
    let mut tree = NodeTree::new();
    let root = scene.instantiate(&classes(), &mut tree).unwrap();
    let map = tree.get_as::<TileMap>(root).unwrap();
    assert_eq!(map.name(), "Board");
    assert_eq!((map.width(), map.height()), (5, 3));
    assert_eq!(map.cell(2, 1), Some('L'));
    assert_eq!(map.cell_size, Vec2::splat(48.0));
    assert_eq!(map.position, Vec2::new(16.0, 32.0));
    assert_eq!(map.tile_set_path, "tiles/courtyard.etileset");
    assert_eq!(map.z(), -1.0);
}

#[test]
fn a_tile_map_round_trips_through_from_tree_and_resave() {
    let db = classes();
    let scene = PackedScene::from_ron(LEVEL).unwrap();
    let mut tree = NodeTree::new();
    let tracked = scene.instantiate_tracked(&db, &mut tree).unwrap();
    let root = tracked.root;
    let resaved = scene.resave(&tree, root, &db, &tracked.origins).unwrap();
    assert_eq!(
        resaved.to_ron(),
        scene.to_ron(),
        "unchanged resave is lossless"
    );

    tree.get_as_mut::<TileMap>(root)
        .unwrap()
        .set_cell(1, 1, Some('o'));
    let saved = PackedScene::from_tree(&tree, root, &db, "uid://level").unwrap();
    let mut back = NodeTree::new();
    let id = saved.instantiate(&db, &mut back).unwrap();
    let map = back.get_as::<TileMap>(id).unwrap();
    assert_eq!(map.rows(), ["#####", "#oL.#", "#####"]);
    assert_eq!(map.z(), -1.0);
}

#[test]
fn bad_rows_are_ignored_not_fatal() {
    let mut m = board();
    assert!(!m.set_property(
        "rows",
        &ember_core::Variant::Array(vec![ember_core::Variant::Int(3)])
    ));
    assert_eq!(m.rows(), ["####", "#..#", "####"], "unchanged");
    assert!(!m.set_property("rows", &ember_core::Variant::String("##".into())));
    assert!(!m.set_property("nope", &ember_core::Variant::Nil));
}
