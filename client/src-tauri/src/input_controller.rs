use enigo::{
    Coordinate, Enigo, Mouse, Keyboard, Button, Direction, Key, Axis
};
use std::sync::Mutex;

pub struct InputController {
    enigo: Mutex<Enigo>,
}

impl InputController {
    pub fn new() -> Self {
        Self {
            enigo: Mutex::new(Enigo::new(&enigo::Settings::default()).unwrap()),
        }
    }

    /// 移动鼠标到绝对坐标
    pub fn mouse_move(&self, x: i32, y: i32) {
        if let Ok(mut enigo) = self.enigo.lock() {
            let _ = enigo.move_mouse(x, y, Coordinate::Abs);
        }
    }

    /// 鼠标点击
    pub fn mouse_click(&self, button: &str, is_down: bool) {
        if let Ok(mut enigo) = self.enigo.lock() {
            let btn = match button {
                "left" => Button::Left,
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => return,
            };

            let direction = if is_down { Direction::Press } else { Direction::Release };
            let _ = enigo.button(btn, direction);
        }
    }

    /// 鼠标滚轮
    pub fn mouse_scroll(&self, x: i32, y: i32) {
        if let Ok(mut enigo) = self.enigo.lock() {
            if y != 0 {
                let _ = enigo.scroll(y, Axis::Vertical);
            }
            if x != 0 {
                let _ = enigo.scroll(x, Axis::Horizontal);
            }
        }
    }

    /// 键盘按键
    pub fn key_event(&self, key_code: &str, is_down: bool) {
        if let Ok(mut enigo) = self.enigo.lock() {
            let key = Self::map_key(key_code);
            let direction = if is_down { Direction::Press } else { Direction::Release };
            let _ = enigo.key(key, direction);
        }
    }

    /// 将前端传来的按键字符串映射为 Enigo 的 Key 枚举
    fn map_key(key_code: &str) -> Key {
        match key_code {
            "Enter" => Key::Return,
            "Backspace" => Key::Backspace,
            "Tab" => Key::Tab,
            "Escape" => Key::Escape,
            "Space" => Key::Space,
            "ArrowUp" => Key::UpArrow,
            "ArrowDown" => Key::DownArrow,
            "ArrowLeft" => Key::LeftArrow,
            "ArrowRight" => Key::RightArrow,
            "Shift" => Key::Shift,
            "Control" => Key::Control,
            "Alt" => Key::Alt,
            "Meta" => Key::Meta,
            "Delete" => Key::Delete,
            "Home" => Key::Home,
            "End" => Key::End,
            "PageUp" => Key::PageUp,
            "PageDown" => Key::PageDown,
            // 对于普通字符，使用 Unicode
            c if c.chars().count() == 1 => Key::Unicode(c.chars().next().unwrap()),
            _ => Key::Unicode('?'), // 未知按键
        }
    }
}
