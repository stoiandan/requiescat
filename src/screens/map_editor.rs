use crate::models::Grave;

struct MapEditorState {
    graves: Vec<Grave>,
    mouse_state: MouseState,
}



impl Default for MapEditorState {
    fn default() -> Self {
        Self { graves: vec![], mouse_state: Default::default() }
    }
}





#[derive(Default)]
struct MouseState {
    is_left_pressed: bool,
    pressed_at: Option<(f32, f32)>,
    released_at: Option<(f32, f32)>,
    position: (f32, f32),
}