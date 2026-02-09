use bevy::prelude::*;
use bevy::text::TextFont;
use super::{ButtonLabel,MARGIN};

pub fn title(asset_server: Res<AssetServer>, size: Vec2) -> impl Bundle {
    let title = asset_server.load("images/menu/title.png");
    (
        ImageNode::new(title),
        Node {
            width: px(size.x),
            height: px(size.y),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    )
}



pub fn button(
    label: &'static str,
    img: Handle<Image>,
    font: Handle<Font>,
    font_size: f32,
    size: Vec2,
) -> impl Bundle {
    (
        Button,
        ImageNode::new(img),
        ButtonLabel(label),
        Node {
            width: px(size.x),
            height: px(size.y),
            margin: UiRect::bottom(MARGIN),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::BLACK),
                Node {
                     top: px(-12),
                    position_type: PositionType::Absolute,
                    margin: UiRect::top(px(4)),
                    ..default()
                },
            ),
            (
                Text::new(label),
                TextFont {
                    font,
                    font_size,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    top: px(-12),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            )
        ],
    )
}

/// Generic fullscreen menu background spawner
pub fn menu_bg<S: States, M: Component>(
    bg_img: Handle<Image>,
    state: S,
    marker: M,
    justify: JustifyContent,
    align: AlignItems,
) -> impl Bundle {
    (
        DespawnOnExit(state),
        Node {
            // fill the entire window
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: align,
            justify_content: justify,
            padding: UiRect::all(MARGIN),
            row_gap: MARGIN,
            ..Default::default()
        },
        ImageNode::new(bg_img),
        marker,
    )
}
