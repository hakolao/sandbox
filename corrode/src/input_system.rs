#![allow(dead_code)]
use std::{collections::HashMap, hash::Hash};

use cgmath::Vector2;
use winit::event::{
    ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};

// A cheap copy of https://github.com/lowenware/dotrix/blob/b3658a3ca7aa7b576414a3b2ccc49a0de41bccc6/dotrix_core/src/input.rs
// To handle inputs in a more consistent manner

/// Input button abstraction
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum InputButton {
    Key(VirtualKeyCode),
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseOther(u8),
}

/// State of a button
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum State {
    Activated,
    Held,
    Deactivated,
}

/// Input event abstraction
#[derive(Debug)]
pub enum InputEvent {
    #[allow(dead_code)]
    Copy,
    #[allow(dead_code)]
    Cut,
    Key(KeyEvent),
    Text(String),
}

/// Key input event
#[derive(Debug)]
pub struct KeyEvent {
    pub key_code: VirtualKeyCode,
    pub pressed: bool,
    pub modifiers: ModifiersState,
}

/// Input System
#[derive(Debug)]
pub struct InputSystem<T> {
    mapper: Mapper<T>,
    button_states: HashMap<InputButton, State>,
    mouse_scroll_delta: f32,
    mouse_position: Option<Vector2<f32>>,
    last_mouse_position: Option<Vector2<f32>>,
    window_size: [u32; 2],
    target_window: Option<egui::Id>,
    pub events: Vec<InputEvent>,
    pub modifiers: ModifiersState,
}

impl<T: Hash + Eq + Copy + 'static> InputSystem<T> {
    /// Service constructor from [`ActionMapper`]
    pub(crate) fn new() -> Self {
        Self {
            mapper: Mapper::<T>::new(),
            button_states: HashMap::new(),
            mouse_scroll_delta: 0.0,
            mouse_position: None,
            last_mouse_position: None,
            target_window: None,
            window_size: [1; 2],
            events: vec![],
            modifiers: ModifiersState::default(),
        }
    }

    #[allow(dead_code)]
    pub fn target_window(&self) -> Option<egui::Id> {
        self.target_window
    }

    #[allow(dead_code)]
    pub fn set_target_window(&mut self, window_id: Option<egui::Id>) {
        self.target_window = window_id;
    }

    pub fn action_mapped(&self, action: T) -> Option<&InputButton> {
        self.mapper.get_button(action)
    }

    /// Returns the status of the mapped action.
    pub fn action_state(&self, action: T) -> Option<State> {
        if let Some(button) = self.action_mapped(action) {
            if let Some(state) = self.button_states.get(button) {
                return Some(*state);
            }
        }
        None
    }

    /// Returns the status of the raw input
    #[allow(dead_code)]
    pub fn button_state(&self, button: InputButton) -> Option<State> {
        self.button_states.get(&button).copied()
    }

    /// Checks if mapped action button is pressed
    #[allow(dead_code)]
    pub fn is_action_activated(&self, action: T) -> bool {
        self.action_state(action)
            .map(|state| state == State::Activated)
            .unwrap_or(false)
    }

    /// Checks if mapped action button is released
    #[allow(dead_code)]
    pub fn is_action_deactivated(&self, action: T) -> bool {
        self.action_state(action)
            .map(|state| state == State::Deactivated)
            .unwrap_or(false)
    }

    /// Checks if mapped button is pressed or hold
    pub fn is_action_held(&self, action: T) -> bool {
        self.action_state(action)
            .map(|state| state == State::Held || state == State::Activated)
            .unwrap_or(false)
    }

    /// Get input mapper reference
    pub fn mapper(&self) -> &Mapper<T> {
        &self.mapper
    }

    /// Get mutatable mapper reference
    pub fn mapper_mut(&mut self) -> &mut Mapper<T> {
        &mut self.mapper
    }

    /// Mouse scroll delta
    #[allow(dead_code)]
    pub fn mouse_scroll(&self) -> f32 {
        self.mouse_scroll_delta
    }

    /// Current mouse position in pixel coordinates. The top-left of the window is at (0, 0)
    #[allow(dead_code)]
    pub fn mouse_position(&self) -> Option<&Vector2<f32>> {
        self.mouse_position.as_ref()
    }

    /// Difference of the mouse position from the last frame in pixel coordinates
    ///
    /// The top-left of the window is at (0, 0).
    #[allow(dead_code)]
    pub fn mouse_delta(&self) -> Vector2<f32> {
        self.last_mouse_position
            .map(|p| self.mouse_position.unwrap() - p)
            .unwrap_or_else(|| Vector2::new(0.0, 0.0))
    }

    /// Normalized mouse position
    ///
    /// The top-left of the window is at (0, 0), bottom-right at (1, 1)
    #[allow(dead_code)]
    pub fn mouse_position_normalized(&self) -> Vector2<f32> {
        let (x, y) = self
            .mouse_position
            .as_ref()
            .map(|p| {
                (
                    (p.x / self.window_size[0] as f32).clamp(0.0, 1.0),
                    (p.y / self.window_size[1] as f32).clamp(0.0, 1.0),
                )
            })
            .unwrap_or((0.0, 0.0));

        Vector2::new(x, y)
    }

    /// Normalized mouse position
    ///
    /// The top-left of the window is at (0, 0), bottom-right at (1, 1)
    #[allow(dead_code)]
    pub fn last_mouse_position_normalized(&self) -> Vector2<f32> {
        let (x, y) = self
            .last_mouse_position
            .as_ref()
            .map(|p| {
                (
                    (p.x / self.window_size[0] as f32).clamp(0.0, 1.0),
                    (p.y / self.window_size[1] as f32).clamp(0.0, 1.0),
                )
            })
            .unwrap_or((0.0, 0.0));

        Vector2::new(x, y)
    }

    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = [width, height]
    }

    /// This method must be called at the end of frame to update states from events
    pub fn reset(&mut self) {
        if let Some(mouse_position) = self.mouse_position.as_ref() {
            self.last_mouse_position = Some(*mouse_position);
        }
        self.mouse_scroll_delta = 0.0;

        self.button_states.retain(|_btn, state| match state {
            State::Activated => {
                *state = State::Held;
                true
            }
            State::Deactivated => false,
            _ => true,
        });

        self.events.clear();
    }

    /// Handles input event
    pub fn on_event<E>(&mut self, event: &winit::event::Event<E>) {
        if let winit::event::Event::WindowEvent {
            event, ..
        } = event
        {
            match event {
                WindowEvent::KeyboardInput {
                    input, ..
                } => self.on_keyboard_event(input),
                WindowEvent::MouseInput {
                    state,
                    button,
                    ..
                } => self.on_mouse_click_event(*state, *button),
                WindowEvent::CursorMoved {
                    position, ..
                } => self.on_cursor_moved_event(position),
                WindowEvent::MouseWheel {
                    delta, ..
                } => self.on_mouse_wheel_event(delta),
                WindowEvent::Resized(size) => self.update_window_size(size.width, size.height),
                WindowEvent::ScaleFactorChanged {
                    new_inner_size, ..
                } => self.update_window_size(new_inner_size.width, new_inner_size.height),
                WindowEvent::ModifiersChanged(input) => self.modifiers = *input,
                WindowEvent::ReceivedCharacter(chr) => {
                    if is_printable(*chr) && !self.modifiers.ctrl() && !self.modifiers.logo() {
                        if let Some(InputEvent::Text(text)) = self.events.last_mut() {
                            text.push(*chr);
                        } else {
                            self.events.push(InputEvent::Text(chr.to_string()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_cursor_moved_event(&mut self, position: &winit::dpi::PhysicalPosition<f64>) {
        self.mouse_position = Some(Vector2::new(position.x as f32, position.y as f32));
    }

    fn on_keyboard_event(&mut self, input: &KeyboardInput) {
        if let Some(key_code) = input.virtual_keycode {
            self.events.push(InputEvent::Key(KeyEvent {
                key_code,
                modifiers: self.modifiers,
                pressed: input.state == ElementState::Pressed,
            }));
            self.on_button_state(InputButton::Key(key_code), input.state);
        }
    }

    fn on_button_state(&mut self, btn: InputButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if *self.button_states.get(&btn).unwrap_or(&State::Deactivated)
                    == State::Deactivated
                {
                    self.button_states.insert(btn, State::Activated);
                }
            }
            ElementState::Released => {
                self.button_states.insert(btn, State::Deactivated);
            }
        }
    }

    fn on_mouse_click_event(&mut self, state: ElementState, mouse_btn: winit::event::MouseButton) {
        let btn: InputButton = match mouse_btn {
            MouseButton::Left => InputButton::MouseLeft,
            MouseButton::Right => InputButton::MouseRight,
            MouseButton::Middle => InputButton::MouseMiddle,
            MouseButton::Other(num) => InputButton::MouseOther(num as u8),
        };

        self.on_button_state(btn, state);
    }

    fn on_mouse_wheel_event(&mut self, delta: &MouseScrollDelta) {
        let change = match delta {
            MouseScrollDelta::LineDelta(_x, y) => *y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
        };
        self.mouse_scroll_delta += change;
    }
}

#[derive(Debug)]
pub struct Mapper<T> {
    map: HashMap<T, InputButton>,
}

impl<T> Mapper<T>
where
    T: Copy + Eq + std::hash::Hash,
{
    /// Constructs new [`Mapper`]
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Add a new action to mapper. If action already exists, it will be overridden
    #[allow(dead_code)]
    pub fn add_action(&mut self, action: T, button: InputButton) {
        self.map.insert(action, button);
    }

    /// Add multiple actions to mapper. Existing actions will be overridden
    #[allow(dead_code)]
    pub fn add_actions(&mut self, actions: &Vec<(T, InputButton)>) {
        for (action, button) in actions.iter() {
            self.map.insert(*action, *button);
        }
    }

    /// Get button that is bound to that action
    pub fn get_button(&self, action: T) -> Option<&InputButton> {
        self.map.get(&action)
    }

    /// Remove action from mapper
    #[allow(dead_code)]
    pub fn remove_action(&mut self, action: T) {
        self.map.remove(&action);
    }

    /// Remove multiple actions from mapper
    #[allow(dead_code)]
    pub fn remove_actions(&mut self, actions: Vec<T>) {
        for action in actions.iter() {
            self.map.remove(action);
        }
    }

    /// Removes all actions and set new ones
    pub fn set(&mut self, actions: &Vec<(T, InputButton)>) {
        self.map.clear();
        self.add_actions(actions);
    }
}

#[inline]
fn is_printable(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);
    !is_in_private_use_area && !chr.is_ascii_control()
}
